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
    Refs {
        base: &'a str,
        head: &'a str,
    },
    /// A base ref compared against the *working tree* (`git diff <base>`), so the
    /// review covers everything since `base` — commits made on top of it **and**
    /// uncommitted edits. Used by `review --since-snapshot` to capture a whole
    /// agent run from the marker the user took beforehand.
    SinceRef {
        base: &'a str,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChangedFile {
    pub path: PathBuf,
    pub status: ChangeStatus,
    pub ranges: Vec<ChangedRange>,
    /// Per-hunk added/removed line content. Internal to the review layer's
    /// content-based signals; never serialized into the JSON report (`ranges`
    /// already carries the public changed-line view).
    #[serde(skip)]
    pub hunks: Vec<DiffHunk>,
}

/// One diff hunk's added and removed line text. `--unified=0` is used, so every
/// body line is either an addition or a removal — there are no context lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffHunk {
    /// Added-line range in the post-change file. `None` for a pure-deletion
    /// hunk (`@@ -a,b +c,0 @@`), which carries only removed lines.
    pub new_range: Option<ChangedRange>,
    /// Removed-line range in the pre-change file. `None` for a pure-addition
    /// hunk (`@@ -0,0 +c,d @@`), which carries only added lines.
    pub old_range: Option<ChangedRange>,
    /// Lines added in this hunk, without the leading `+`.
    pub added_lines: Vec<String>,
    /// Lines removed in this hunk, without the leading `-`.
    pub removed_lines: Vec<String>,
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

pub fn validate_git_ref(reference: &str) -> Result<(), GitDiffError> {
    if reference.starts_with('-') {
        return Err(GitDiffError::GitCommandFailed {
            command: "validate_git_ref".to_string(),
            stderr: format!(
                "Invalid git reference: '{}' cannot start with a hyphen",
                reference
            ),
        });
    }
    if reference
        .chars()
        .any(|c| c.is_whitespace() || c.is_control())
    {
        return Err(GitDiffError::GitCommandFailed {
            command: "validate_git_ref".to_string(),
            stderr: format!(
                "Invalid git reference: '{}' cannot contain whitespace or control characters",
                reference
            ),
        });
    }
    Ok(())
}

pub fn load_changed_files(
    repo_root: &Path,
    target: DiffTarget<'_>,
    pathspec: Option<&str>,
) -> Result<Vec<ChangedFile>, GitDiffError> {
    match target {
        DiffTarget::WorkingTree => {}
        DiffTarget::Refs { base, head } => {
            validate_git_ref(base)?;
            validate_git_ref(head)?;
        }
        DiffTarget::SinceRef { base } => {
            validate_git_ref(base)?;
        }
    }

    let mut files = match target {
        DiffTarget::WorkingTree => parse_diff(&git_diff_against_head(repo_root, pathspec)?),
        DiffTarget::Refs { base, head } => {
            parse_diff(&git_diff_between_refs(repo_root, base, head, pathspec)?)
        }
        DiffTarget::SinceRef { base } => {
            parse_diff(&git_diff_since_ref(repo_root, base, pathspec)?)
        }
    };

    // Both targets that end at the working tree must also pick up untracked files,
    // since a ref-vs-worktree `git diff` only reports tracked changes.
    if matches!(
        target,
        DiffTarget::WorkingTree | DiffTarget::SinceRef { .. }
    ) {
        files.extend(load_untracked_files(repo_root, pathspec)?);
    }

    files.retain(|file| !is_repopilot_internal_path(&file.path));
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(files)
}

/// Read a file's content at a git revision (`git show <reference>:<path>`).
///
/// Returns `None` when the path does not exist at that revision or git fails;
/// callers treat an absent side as "no content to compare". `path` must be
/// repo-relative with forward slashes (use [`ChangedFile::path_string`]).
pub(crate) fn git_show(repo_root: &Path, reference: &str, path: &str) -> Option<String> {
    validate_git_ref(reference).ok()?;
    let spec = format!("{reference}:{path}");
    git_output(repo_root, &["show", spec.as_str()], "git show").ok()
}

pub fn parse_diff(diff: &str) -> Vec<ChangedFile> {
    let mut files = Vec::new();
    let mut current: Option<ChangedFile> = None;
    let mut hunk: Option<DiffHunk> = None;

    for line in diff.lines() {
        if let Some((_, new_path)) = parse_diff_git_line(line) {
            if let Some(file) = current.take() {
                files.push(finalize_file(file, hunk.take()));
            }

            current = Some(ChangedFile {
                path: PathBuf::from(new_path),
                status: ChangeStatus::Modified,
                ranges: Vec::new(),
                hunks: Vec::new(),
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

        if line.starts_with("@@") {
            if let Some(done) = hunk.take() {
                file.hunks.push(done);
            }
            hunk = Some(DiffHunk {
                new_range: parse_hunk_added_range(line),
                old_range: parse_hunk_removed_range(line),
                added_lines: Vec::new(),
                removed_lines: Vec::new(),
            });
            continue;
        }

        if let Some(active) = hunk.as_mut() {
            if let Some(added) = line.strip_prefix('+') {
                active.added_lines.push(added.to_string());
            } else if let Some(removed) = line.strip_prefix('-') {
                active.removed_lines.push(removed.to_string());
            }
        }
    }

    if let Some(file) = current {
        files.push(finalize_file(file, hunk.take()));
    }

    files
}

/// Push the trailing hunk (if any) and derive the public `ranges` view from the
/// captured hunks so existing range consumers see exactly what they did before.
fn finalize_file(mut file: ChangedFile, trailing: Option<DiffHunk>) -> ChangedFile {
    if let Some(done) = trailing {
        file.hunks.push(done);
    }
    file.ranges = file
        .hunks
        .iter()
        .filter_map(|hunk| hunk.new_range)
        .collect();
    file
}

include!("diff/helpers.rs");

#[cfg(test)]
mod tests;
