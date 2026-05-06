use crate::baseline::model::Baseline;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum BaselineWriteError {
    AlreadyExists { path: PathBuf },
    Io { path: PathBuf, source: io::Error },
    Json { source: serde_json::Error },
}

impl fmt::Display for BaselineWriteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BaselineWriteError::AlreadyExists { path } => write!(
                formatter,
                "Baseline already exists at {}. Use `repopilot baseline create --force` to overwrite it.",
                path.display()
            ),
            BaselineWriteError::Io { path, source } => {
                write!(
                    formatter,
                    "Failed to write baseline file: {}\nReason: {source}",
                    path.display()
                )
            }
            BaselineWriteError::Json { source } => {
                write!(formatter, "Failed to serialize baseline JSON: {source}")
            }
        }
    }
}

impl std::error::Error for BaselineWriteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BaselineWriteError::Io { source, .. } => Some(source),
            BaselineWriteError::Json { source } => Some(source),
            BaselineWriteError::AlreadyExists { .. } => None,
        }
    }
}

pub fn write_baseline(
    baseline: &Baseline,
    output_path: &Path,
    force: bool,
) -> Result<(), BaselineWriteError> {
    if let Some(parent) = output_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|source| BaselineWriteError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let content = serde_json::to_string_pretty(baseline)
        .map_err(|source| BaselineWriteError::Json { source })?;

    if force {
        return fs::write(output_path, content).map_err(|source| BaselineWriteError::Io {
            path: output_path.to_path_buf(),
            source,
        });
    }

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output_path)
        .map_err(|source| match source.kind() {
            io::ErrorKind::AlreadyExists => BaselineWriteError::AlreadyExists {
                path: output_path.to_path_buf(),
            },
            _ => BaselineWriteError::Io {
                path: output_path.to_path_buf(),
                source,
            },
        })?;

    file.write_all(content.as_bytes())
        .map_err(|source| BaselineWriteError::Io {
            path: output_path.to_path_buf(),
            source,
        })
}
