use std::io;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("JSON serialization/deserialization failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("YAML serialization/deserialization failed: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Glob pattern matching failed: {0}")]
    Glob(#[from] globset::Error),

    #[error("ignore walker walk failed: {0}")]
    Walk(#[from] ignore::Error),

    #[error("Git command execution failed: {0}")]
    Git(String),

    #[error("TOML serialization/deserialization failed: {0}")]
    Toml(#[from] toml::de::Error),
}
