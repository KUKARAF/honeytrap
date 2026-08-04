pub mod functions;
pub mod manifest;
pub mod validate;

use crate::errors::TemplateError;
use manifest::Manifest;
use minijinja::Environment;
use std::collections::HashMap;
use std::path::Path;

pub struct TemplateStore {
    pub env: Environment<'static>,
    /// request path -> minijinja template name (== on-disk filename)
    path_map: HashMap<String, String>,
}

impl TemplateStore {
    pub fn load(dir: &Path) -> Result<TemplateStore, TemplateError> {
        let mut env = Environment::new();
        functions::register(&mut env);

        let mut path_map = HashMap::new();
        let mut known_templates = Vec::new();

        let entries =
            std::fs::read_dir(dir).map_err(|e| TemplateError::ReadDir(dir.to_path_buf(), e))?;
        for entry in entries {
            let entry = entry.map_err(|e| TemplateError::ReadDir(dir.to_path_buf(), e))?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let filename = match path.file_name().and_then(|n| n.to_str()) {
                Some(f) if f.ends_with(".tmpl") => f.to_string(),
                _ => continue,
            };
            let contents = std::fs::read_to_string(&path)
                .map_err(|e| TemplateError::ReadFile(path.clone(), e))?;

            env.add_template_owned(filename.clone(), contents)
                .map_err(|e| TemplateError::Compile {
                    name: filename.clone(),
                    source: e,
                })?;

            if let Some(request_path) = manifest::filename_to_path(&filename) {
                path_map.insert(request_path, filename.clone());
            }
            known_templates.push(filename);
        }

        let manifest = Manifest::load(dir)?;
        for (alias_path, target_filename) in manifest.aliases {
            if !known_templates.contains(&target_filename) {
                return Err(TemplateError::UnknownManifestTarget {
                    alias: alias_path,
                    target: target_filename,
                });
            }
            path_map.insert(alias_path, target_filename);
        }

        Ok(TemplateStore { env, path_map })
    }

    pub fn resolve_path(&self, request_path: &str) -> Option<&str> {
        self.path_map.get(request_path).map(String::as_str)
    }

    pub fn template_names(&self) -> impl Iterator<Item = &str> {
        self.path_map.values().map(String::as_str)
    }
}
