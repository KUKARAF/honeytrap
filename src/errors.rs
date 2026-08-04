use std::path::PathBuf;

#[derive(thiserror::Error, Debug)]
pub enum TemplateError {
    #[error("failed to read templates directory {0}: {1}")]
    ReadDir(PathBuf, std::io::Error),
    #[error("failed to read template file {0}: {1}")]
    ReadFile(PathBuf, std::io::Error),
    #[error("failed to parse manifest.toml: {0}")]
    Manifest(String),
    #[error("manifest alias {alias:?} references unknown template {target:?}")]
    UnknownManifestTarget { alias: String, target: String },
    #[error("failed to compile template {name:?}: {source}")]
    Compile {
        name: String,
        #[source]
        source: minijinja::Error,
    },
}

#[derive(thiserror::Error, Debug)]
pub enum RenderError {
    #[error("unknown template {0:?}")]
    UnknownTemplate(String),
    #[error("failed to render template {name:?}: {source}")]
    Render {
        name: String,
        #[source]
        source: minijinja::Error,
    },
}

#[derive(Debug)]
pub struct StartupValidationError {
    pub max_bytes: usize,
    pub details: Vec<String>,
}

impl std::fmt::Display for StartupValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "{} template(s) exceed --max-bytes={}",
            self.details.len(),
            self.max_bytes
        )?;
        for line in &self.details {
            writeln!(f, "  - {line}")?;
        }
        Ok(())
    }
}

impl std::error::Error for StartupValidationError {}
