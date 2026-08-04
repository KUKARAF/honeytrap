use crate::errors::TemplateError;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Deserialize, Default)]
pub struct Manifest {
    #[serde(default)]
    pub aliases: HashMap<String, String>,
}

impl Manifest {
    /// Loads `manifest.toml` from the templates directory if present;
    /// returns an empty (no-alias) manifest if the file doesn't exist.
    pub fn load(dir: &Path) -> Result<Manifest, TemplateError> {
        let manifest_path = dir.join("manifest.toml");
        if !manifest_path.exists() {
            return Ok(Manifest::default());
        }
        let contents = std::fs::read_to_string(&manifest_path)
            .map_err(|e| TemplateError::ReadFile(manifest_path.clone(), e))?;
        toml::from_str(&contents).map_err(|e| TemplateError::Manifest(e.to_string()))
    }
}

/// Derives the on-disk template filename for a request path, e.g.
/// `/.aws/credentials` -> `.aws__credentials.tmpl`. Lossless and invertible:
/// dropping a correctly-named file is sufficient to serve a new path, no
/// registration/recompile required.
#[allow(dead_code)] // documents the invertible convention used by filename_to_path; exercised in tests
pub fn path_to_filename(request_path: &str) -> String {
    format!(
        "{}.tmpl",
        request_path.trim_start_matches('/').replace('/', "__")
    )
}

/// Reverses `path_to_filename` for files discovered on disk.
pub fn filename_to_path(filename: &str) -> Option<String> {
    let stem = filename.strip_suffix(".tmpl")?;
    Some(format!("/{}", stem.replace("__", "/")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_default_paths() {
        for path in [
            "/.env",
            "/.aws/credentials",
            "/.git/config",
            "/config.json",
            "/wp-config.php",
            "/docker-compose.yml",
            "/.npmrc",
            "/secrets.json",
        ] {
            let filename = path_to_filename(path);
            assert_eq!(filename_to_path(&filename).as_deref(), Some(path));
        }
    }
}
