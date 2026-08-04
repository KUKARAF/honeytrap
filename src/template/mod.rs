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
    /// Sorted, deduplicated list of template names used to deterministically
    /// pick a fallback for an unmatched request path. Sorted so the index a
    /// seed maps to is stable across process restarts.
    fallback_pool: Vec<String>,
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

        let mut fallback_pool: Vec<String> = known_templates;
        fallback_pool.sort();
        fallback_pool.dedup();

        Ok(TemplateStore {
            env,
            path_map,
            fallback_pool,
        })
    }

    pub fn resolve_path(&self, request_path: &str) -> Option<&str> {
        self.path_map.get(request_path).map(String::as_str)
    }

    /// Deterministically pick a template for an unmatched path from the sorted
    /// pool, indexed by the request's seed. Same seed always yields the same
    /// template, including across restarts. Returns `None` only when no
    /// templates are loaded at all.
    pub fn pick_fallback(&self, seed: &[u8; 32]) -> Option<&str> {
        if self.fallback_pool.is_empty() {
            return None;
        }
        let n = self.fallback_pool.len() as u64;
        let idx = (u64::from_le_bytes(seed[0..8].try_into().unwrap()) % n) as usize;
        Some(&self.fallback_pool[idx])
    }

    pub fn template_names(&self) -> impl Iterator<Item = &str> {
        self.path_map.values().map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_pick_is_stable_across_loads() {
        // Loading the store twice must produce the same sorted pool, so the
        // same seed always maps to the same template — even across restarts,
        // where HashMap iteration order would otherwise differ.
        let dir = Path::new("templates");
        let a = TemplateStore::load(dir).expect("load");
        let b = TemplateStore::load(dir).expect("load");
        assert_eq!(a.fallback_pool, b.fallback_pool);
        assert!(!a.fallback_pool.is_empty());

        let seed = [7u8; 32];
        assert_eq!(a.pick_fallback(&seed), b.pick_fallback(&seed));
    }
}
