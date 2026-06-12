use crate::config::defaults::CONFIG_FILE_NAME;
use crate::config::model::RepoPilotConfig;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub fn load_default_config() -> Result<RepoPilotConfig, ConfigError> {
    let start = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    match discover_config_path(&start) {
        Some(path) => load_optional_config(&path),
        None => Ok(RepoPilotConfig::default()),
    }
}

/// Walks from `start_dir` up to the enclosing git root looking for
/// `repopilot.toml`, returning the first match. The search stops at the
/// directory that holds `.git` (inclusive — a config beside `.git` is still
/// found) or at the filesystem root, so it never escapes the repository. An
/// explicit `--config <path>` bypasses discovery entirely.
pub fn discover_config_path(start_dir: &Path) -> Option<PathBuf> {
    let mut dir = start_dir;
    loop {
        let candidate = dir.join(CONFIG_FILE_NAME);
        if candidate.is_file() {
            return Some(candidate);
        }
        // `.git` may be a directory (normal clone) or a file (worktree /
        // submodule), so test existence rather than file type.
        if dir.join(".git").exists() {
            return None;
        }
        dir = dir.parent()?;
    }
}

pub fn load_optional_config(path: &Path) -> Result<RepoPilotConfig, ConfigError> {
    match fs::read_to_string(path) {
        Ok(contents) => parse_config(&contents, Some(path)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(RepoPilotConfig::default()),
        Err(error) => Err(ConfigError::Read {
            path: path.to_path_buf(),
            source: error,
        }),
    }
}

pub fn parse_config(contents: &str, path: Option<&Path>) -> Result<RepoPilotConfig, ConfigError> {
    toml::from_str(contents).map_err(|source| ConfigError::Parse {
        path: path.map(Path::to_path_buf),
        source,
    })
}

#[derive(Debug)]
pub enum ConfigError {
    Read {
        path: PathBuf,
        source: io::Error,
    },
    Parse {
        path: Option<PathBuf>,
        source: toml::de::Error,
    },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(
                    formatter,
                    "failed to read config {}: {source}",
                    path.display()
                )
            }
            Self::Parse { path, source } => {
                if let Some(path) = path {
                    write!(formatter, "invalid config {}: {source}", path.display())
                } else {
                    write!(formatter, "invalid config: {source}")
                }
            }
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::Parse { source, .. } => Some(source),
        }
    }
}
