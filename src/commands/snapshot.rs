use crate::cli::SnapshotOptions;
use chrono::{SecondsFormat, Utc};
use repopilot::review::diff::resolve_git_root;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Current on-disk schema for `.repopilot/snapshot.json`.
const SNAPSHOT_SCHEMA_VERSION: u32 = 1;

/// A "before the agent starts" marker: the repository `HEAD` at the moment
/// `repopilot snapshot` ran, plus whether the working tree was already dirty.
/// `repopilot review --since-snapshot` diffs this `head` against the current
/// working tree to review the whole run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub schema_version: u32,
    /// The `HEAD` commit sha recorded when the snapshot was taken.
    pub head: String,
    /// Whether the working tree already had uncommitted edits at snapshot time.
    pub dirty: bool,
    /// RFC 3339 timestamp of when the snapshot was taken.
    pub created_at: String,
}

#[derive(Debug)]
pub enum SnapshotError {
    Git(repopilot::review::diff::GitDiffError),
    NoCommits,
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },
    Missing {
        path: PathBuf,
    },
    InvalidSnapshot {
        path: PathBuf,
        reason: String,
    },
    UnsupportedSchemaVersion {
        found: u32,
    },
}

impl fmt::Display for SnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SnapshotError::Git(error) => write!(formatter, "{error}"),
            SnapshotError::NoCommits => write!(
                formatter,
                "the repository has no commits yet; commit at least once before taking a snapshot"
            ),
            SnapshotError::Io { path, source } => {
                write!(formatter, "failed to access {}: {source}", path.display())
            }
            SnapshotError::Json { path, source } => write!(
                formatter,
                "failed to parse snapshot file {}: {source}",
                path.display()
            ),
            SnapshotError::Missing { path } => write!(
                formatter,
                "no snapshot found at {}. Run `repopilot snapshot` before the change you want to review.",
                path.display()
            ),
            SnapshotError::InvalidSnapshot { path, reason } => write!(
                formatter,
                "invalid snapshot file {}: {reason}",
                path.display()
            ),
            SnapshotError::UnsupportedSchemaVersion { found } => write!(
                formatter,
                "unsupported snapshot schema version: {found}; supported version: {SNAPSHOT_SCHEMA_VERSION}"
            ),
        }
    }
}

impl Error for SnapshotError {}

impl From<repopilot::review::diff::GitDiffError> for SnapshotError {
    fn from(error: repopilot::review::diff::GitDiffError) -> Self {
        SnapshotError::Git(error)
    }
}

pub fn run(options: SnapshotOptions) -> Result<(), Box<dyn Error>> {
    let repo_root = resolve_git_root(&options.path)?;
    let head = head_sha(&repo_root)?;
    let dirty = is_dirty(&repo_root)?;
    let snapshot = Snapshot {
        schema_version: SNAPSHOT_SCHEMA_VERSION,
        head: head.clone(),
        dirty,
        created_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
    };

    let path = snapshot_path(&repo_root);
    write_snapshot(&snapshot, &path)?;
    print_snapshot_summary(&path, &snapshot);
    Ok(())
}

/// Read the snapshot recorded for the repository containing `scan_path`.
pub fn read_snapshot(scan_path: &Path) -> Result<Snapshot, SnapshotError> {
    let repo_root = resolve_git_root(scan_path)?;
    let path = snapshot_path(&repo_root);
    if !path.exists() {
        return Err(SnapshotError::Missing { path });
    }

    let content = fs::read_to_string(&path).map_err(|source| SnapshotError::Io {
        path: path.clone(),
        source,
    })?;
    let snapshot: Snapshot =
        serde_json::from_str(&content).map_err(|source| SnapshotError::Json {
            path: path.clone(),
            source,
        })?;
    validate_snapshot(&snapshot, &path)?;
    Ok(snapshot)
}

fn snapshot_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".repopilot/snapshot.json")
}

fn write_snapshot(snapshot: &Snapshot, path: &Path) -> Result<(), SnapshotError> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|source| SnapshotError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let content = serde_json::to_string_pretty(snapshot).map_err(|source| SnapshotError::Json {
        path: path.to_path_buf(),
        source,
    })?;
    fs::write(path, content).map_err(|source| SnapshotError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn validate_snapshot(snapshot: &Snapshot, path: &Path) -> Result<(), SnapshotError> {
    if snapshot.schema_version != SNAPSHOT_SCHEMA_VERSION {
        return Err(SnapshotError::UnsupportedSchemaVersion {
            found: snapshot.schema_version,
        });
    }

    if snapshot.head.trim().is_empty() {
        return Err(SnapshotError::InvalidSnapshot {
            path: path.to_path_buf(),
            reason: "missing required field `head`".to_string(),
        });
    }

    if snapshot.created_at.trim().is_empty() {
        return Err(SnapshotError::InvalidSnapshot {
            path: path.to_path_buf(),
            reason: "missing required field `created_at`".to_string(),
        });
    }

    Ok(())
}

fn head_sha(repo_root: &Path) -> Result<String, SnapshotError> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_root)
        .output()
        .map_err(|source| SnapshotError::Io {
            path: repo_root.to_path_buf(),
            source,
        })?;

    if !output.status.success() {
        return Err(SnapshotError::NoCommits);
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn is_dirty(repo_root: &Path) -> Result<bool, SnapshotError> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo_root)
        .output()
        .map_err(|source| SnapshotError::Io {
            path: repo_root.to_path_buf(),
            source,
        })?;

    Ok(!String::from_utf8_lossy(&output.stdout).trim().is_empty())
}

fn print_snapshot_summary(path: &Path, snapshot: &Snapshot) {
    println!("RepoPilot Snapshot");
    println!();
    println!("Snapshot written to: {}", path.display());
    println!("HEAD: {}", snapshot.head);
    println!(
        "Working tree at snapshot time: {}",
        if snapshot.dirty { "dirty" } else { "clean" }
    );
    println!();
    println!("Review everything since this point with:");
    println!("  repopilot review --since-snapshot");
}
