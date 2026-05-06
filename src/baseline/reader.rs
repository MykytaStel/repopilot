use crate::baseline::model::{BASELINE_SCHEMA_VERSION, Baseline};
use crate::findings::types::Severity;
use serde_json::Value;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum BaselineReadError {
    Missing {
        path: PathBuf,
    },
    Empty {
        path: PathBuf,
    },
    InvalidJson {
        path: PathBuf,
        source: serde_json::Error,
    },
    InvalidBaseline {
        path: PathBuf,
        reason: String,
    },
    UnsupportedSchemaVersion {
        found: u32,
    },
    Io {
        path: PathBuf,
        source: io::Error,
    },
}

impl fmt::Display for BaselineReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BaselineReadError::Missing { path } => {
                write!(
                    formatter,
                    "Failed to read baseline file: {}\nReason: file does not exist",
                    path.display()
                )
            }
            BaselineReadError::Empty { path } => {
                write!(
                    formatter,
                    "Failed to read baseline file: {}\nReason: baseline file is empty",
                    path.display()
                )
            }
            BaselineReadError::InvalidJson { path, .. } => {
                write!(
                    formatter,
                    "Failed to read baseline file: {}\nReason: invalid JSON",
                    path.display()
                )
            }
            BaselineReadError::InvalidBaseline { path, reason } => {
                write!(
                    formatter,
                    "Failed to read baseline file: {}\nReason: {reason}",
                    path.display()
                )
            }
            BaselineReadError::UnsupportedSchemaVersion { found } => {
                write!(
                    formatter,
                    "Unsupported baseline schema version: {found}\nSupported version: {BASELINE_SCHEMA_VERSION}"
                )
            }
            BaselineReadError::Io { path, source } => {
                write!(
                    formatter,
                    "Failed to read baseline file: {}\nReason: {source}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for BaselineReadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BaselineReadError::InvalidJson { source, .. } => Some(source),
            BaselineReadError::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

pub fn read_baseline(path: &Path) -> Result<Baseline, BaselineReadError> {
    let content = fs::read_to_string(path).map_err(|source| match source.kind() {
        io::ErrorKind::NotFound => BaselineReadError::Missing {
            path: path.to_path_buf(),
        },
        _ => BaselineReadError::Io {
            path: path.to_path_buf(),
            source,
        },
    })?;

    if content.trim().is_empty() {
        return Err(BaselineReadError::Empty {
            path: path.to_path_buf(),
        });
    }

    let value: Value =
        serde_json::from_str(&content).map_err(|source| BaselineReadError::InvalidJson {
            path: path.to_path_buf(),
            source,
        })?;

    let schema_version = value
        .get("schema_version")
        .and_then(Value::as_u64)
        .ok_or_else(|| BaselineReadError::InvalidBaseline {
            path: path.to_path_buf(),
            reason: "missing required field `schema_version`".to_string(),
        })?;

    if schema_version != u64::from(BASELINE_SCHEMA_VERSION) {
        return Err(BaselineReadError::UnsupportedSchemaVersion {
            found: schema_version as u32,
        });
    }

    let baseline: Baseline =
        serde_json::from_value(value).map_err(|source| BaselineReadError::InvalidBaseline {
            path: path.to_path_buf(),
            reason: source.to_string(),
        })?;

    validate_baseline(&baseline, path)?;

    Ok(baseline)
}

fn validate_baseline(baseline: &Baseline, path: &Path) -> Result<(), BaselineReadError> {
    if baseline.tool.trim().is_empty() {
        return invalid(path, "missing required field `tool`");
    }

    if baseline.created_at.trim().is_empty() {
        return invalid(path, "missing required field `created_at`");
    }

    if baseline.root.trim().is_empty() {
        return invalid(path, "missing required field `root`");
    }

    for finding in &baseline.findings {
        if finding.key.trim().is_empty() {
            return invalid(path, "finding entry is missing required field `key`");
        }

        if finding.rule_id.trim().is_empty() {
            return invalid(path, "finding entry is missing required field `rule_id`");
        }

        if Severity::from_lowercase_label(&finding.severity).is_none() {
            return invalid(path, "finding entry has unsupported `severity`");
        }

        if finding.path.trim().is_empty() {
            return invalid(path, "finding entry is missing required field `path`");
        }

        if finding.message.trim().is_empty() {
            return invalid(path, "finding entry is missing required field `message`");
        }
    }

    Ok(())
}

fn invalid<T>(path: &Path, reason: &str) -> Result<T, BaselineReadError> {
    Err(BaselineReadError::InvalidBaseline {
        path: path.to_path_buf(),
        reason: reason.to_string(),
    })
}
