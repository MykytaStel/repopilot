use serde::Serialize;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffTarget<'a> {
    WorkingTree,
    Refs { base: &'a str, head: &'a str },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChangedFile {
    pub path: PathBuf,
    pub status: ChangeStatus,
    pub ranges: Vec<ChangedRange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Untracked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ChangedRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug)]
pub enum GitDiffError {
    GitNotFound,
    GitCommandFailed { command: String, stderr: String },
    Io(io::Error),
}

impl<'a> DiffTarget<'a> {
    pub fn from_refs(base: Option<&'a str>, head: Option<&'a str>) -> Self {
        match base {
            Some(base) => Self::Refs {
                base,
                head: head.unwrap_or("HEAD"),
            },
            None => Self::WorkingTree,
        }
    }
}

impl ChangedFile {
    pub fn path_string(&self) -> String {
        self.path.to_string_lossy().replace('\\', "/")
    }

    pub fn contains_line(&self, line: usize) -> bool {
        self.ranges
            .iter()
            .any(|range| line >= range.start && line <= range.end)
    }
}

impl fmt::Display for GitDiffError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitDiffError::GitNotFound => write!(
                formatter,
                "git executable was not found; `repopilot review` requires git"
            ),
            GitDiffError::GitCommandFailed { command, stderr } => {
                let message = stderr.trim();
                if message.is_empty() {
                    write!(formatter, "git command failed: {command}")
                } else {
                    write!(formatter, "git command failed: {command}: {message}")
                }
            }
            GitDiffError::Io(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for GitDiffError {}

impl From<io::Error> for GitDiffError {
    fn from(error: io::Error) -> Self {
        if error.kind() == io::ErrorKind::NotFound {
            GitDiffError::GitNotFound
        } else {
            GitDiffError::Io(error)
        }
    }
}

pub fn resolve_git_root(path: &Path) -> Result<PathBuf, GitDiffError> {
    let cwd = if path.is_file() {
        path.parent().unwrap_or_else(|| Path::new("."))
    } else {
        path
    };

    let output = git_output(
        cwd,
        &["rev-parse", "--show-toplevel"],
        "git rev-parse --show-toplevel",
    )?;

    Ok(PathBuf::from(output.trim()))
}

pub fn load_changed_files(
    repo_root: &Path,
    target: DiffTarget<'_>,
    pathspec: Option<&str>,
) -> Result<Vec<ChangedFile>, GitDiffError> {
    let mut files = match target {
        DiffTarget::WorkingTree => parse_diff(&git_diff_against_head(repo_root, pathspec)?),
        DiffTarget::Refs { base, head } => {
            parse_diff(&git_diff_between_refs(repo_root, base, head, pathspec)?)
        }
    };

    if target == DiffTarget::WorkingTree {
        files.extend(load_untracked_files(repo_root, pathspec)?);
    }

    files.retain(|file| !is_repopilot_internal_path(&file.path));
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(files)
}

pub fn parse_diff(diff: &str) -> Vec<ChangedFile> {
    let mut files = Vec::new();
    let mut current: Option<ChangedFile> = None;

    for line in diff.lines() {
        if let Some((_, new_path)) = parse_diff_git_line(line) {
            if let Some(file) = current.take() {
                files.push(file);
            }

            current = Some(ChangedFile {
                path: PathBuf::from(new_path),
                status: ChangeStatus::Modified,
                ranges: Vec::new(),
            });

            continue;
        }

        let Some(file) = current.as_mut() else {
            continue;
        };

        if line.starts_with("new file mode ") {
            file.status = ChangeStatus::Added;
            continue;
        }

        if line.starts_with("deleted file mode ") {
            file.status = ChangeStatus::Deleted;
            continue;
        }

        if line.starts_with("rename from ") {
            file.status = ChangeStatus::Renamed;
            continue;
        }

        if let Some(path) = line.strip_prefix("+++ ") {
            if let Some(path) = normalize_diff_path(path)
                && file.status != ChangeStatus::Deleted
            {
                file.path = PathBuf::from(path);
            }
            continue;
        }

        if let Some(path) = line.strip_prefix("--- ") {
            if let Some(path) = normalize_diff_path(path)
                && file.status == ChangeStatus::Deleted
            {
                file.path = PathBuf::from(path);
            }
            continue;
        }

        if let Some(range) = parse_hunk_added_range(line) {
            file.ranges.push(range);
        }
    }

    if let Some(file) = current {
        files.push(file);
    }

    files
}

include!("diff/helpers.rs");

#[cfg(test)]
mod tests;
