use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

mod batch;
mod error;
mod since_base;
pub(crate) use batch::{BatchedContent, git_show_many};
pub use error::GitDiffError;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnedDiffTarget {
    WorkingTree,
    Refs { base: String, head: String },
    SinceRef { base: String },
}

impl OwnedDiffTarget {
    pub fn from_refs(base: Option<&str>, head: Option<&str>) -> Self {
        match base {
            Some(base) => Self::Refs {
                base: base.to_string(),
                head: head.unwrap_or("HEAD").to_string(),
            },
            None => Self::WorkingTree,
        }
    }

    pub fn as_borrowed(&self) -> DiffTarget<'_> {
        match self {
            Self::WorkingTree => DiffTarget::WorkingTree,
            Self::Refs { base, head } => DiffTarget::Refs { base, head },
            Self::SinceRef { base } => DiffTarget::SinceRef { base },
        }
    }

    pub fn base_ref(&self) -> Option<&str> {
        match self {
            Self::WorkingTree => None,
            Self::Refs { base, .. } | Self::SinceRef { base } => Some(base),
        }
    }
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
    /// Optional text after the hunk marker. Git uses this to expose a nearby
    /// structural header such as `name = "beta"` in Cargo.lock.
    pub header: Option<String>,
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

pub fn resolve_git_root(path: &Path) -> Result<PathBuf, GitDiffError> {
    if !path.exists() {
        return Err(GitDiffError::PathNotFound(path.to_path_buf()));
    }
    let cwd = if path.is_file() {
        path.parent().unwrap_or_else(|| Path::new("."))
    } else {
        path
    };

    let output = git_output(
        cwd,
        &["rev-parse", "--show-toplevel"],
        "git rev-parse --show-toplevel",
    )
    .map_err(|error| match error {
        GitDiffError::GitCommandFailed { ref stderr, .. }
            if stderr.contains("not a git repository") =>
        {
            GitDiffError::NotARepository(path.to_path_buf())
        }
        other => other,
    })?;

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
        DiffTarget::WorkingTree => parse_diff(
            &git_diff_against_head(repo_root, pathspec)
                .map_err(|error| explain_diff_failure(repo_root, target, error))?,
        ),
        DiffTarget::Refs { base, head } => parse_diff(
            &git_diff_between_refs(repo_root, base, head, pathspec)
                .map_err(|error| explain_diff_failure(repo_root, target, error))?,
        ),
        DiffTarget::SinceRef { base } => parse_diff(
            &git_diff_since_ref(repo_root, base, pathspec)
                .map_err(|error| explain_diff_failure(repo_root, target, error))?,
        ),
    };

    // Both targets that end at the working tree must also pick up untracked files,
    // since a ref-vs-worktree `git diff` only reports tracked changes.
    if matches!(
        target,
        DiffTarget::WorkingTree | DiffTarget::SinceRef { .. }
    ) {
        files.extend(load_untracked_files(repo_root, pathspec)?);
    }
    // A snapshot baseline can already hold files that are untracked now.
    if let DiffTarget::SinceRef { base } = target {
        since_base::reconcile_untracked_with_base(repo_root, base, &mut files)?;
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

/// Enumerate repository-relative files for the post-change side of `target`.
///
/// This is file-name plumbing only: it does not read repository contents. Ref
/// reviews use the selected head tree, while worktree-backed reviews include
/// tracked and untracked files that still exist on disk. Callers suppress
/// resolver-backed claims when Git cannot provide an inventory.
pub(crate) fn target_file_inventory(
    repo_root: &Path,
    target: DiffTarget<'_>,
) -> Option<std::collections::HashSet<PathBuf>> {
    let output = match target {
        DiffTarget::Refs { head, .. } => {
            validate_git_ref(head).ok()?;
            git_output(
                repo_root,
                &["ls-tree", "-r", "--name-only", "-z", head, "--"],
                "git ls-tree -r --name-only",
            )
            .ok()?
        }
        DiffTarget::WorkingTree | DiffTarget::SinceRef { .. } => git_output(
            repo_root,
            &[
                "ls-files",
                "--cached",
                "--others",
                "--exclude-standard",
                "-z",
                "--",
            ],
            "git ls-files --cached --others --exclude-standard",
        )
        .ok()?,
    };

    Some(
        output
            .split('\0')
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
            .filter(|path| !is_repopilot_internal_path(path))
            .filter(|path| {
                matches!(target, DiffTarget::Refs { .. }) || repo_root.join(path).is_file()
            })
            .collect(),
    )
}

pub fn parse_diff(diff: &str) -> Vec<ChangedFile> {
    let mut files = Vec::new();
    let mut current: Option<ChangedFile> = None;
    let mut hunk: Option<DiffHunk> = None;

    for line in diff.lines() {
        consume_diff_line(line, &mut files, &mut current, &mut hunk);
    }

    if let Some(file) = current {
        files.push(finalize_file(file, hunk.take()));
    }

    files
}

fn consume_diff_line(
    line: &str,
    files: &mut Vec<ChangedFile>,
    current: &mut Option<ChangedFile>,
    hunk: &mut Option<DiffHunk>,
) {
    if let Some((_, new_path)) = parse_diff_git_line(line) {
        finish_current_file(files, current, hunk.take());
        *current = Some(ChangedFile {
            path: PathBuf::from(new_path),
            status: ChangeStatus::Modified,
            ranges: Vec::new(),
            hunks: Vec::new(),
        });
        return;
    }

    let Some(file) = current.as_mut() else {
        return;
    };
    if update_file_status(line, file) {
        return;
    }
    if update_file_path(line, file) {
        return;
    }
    if line.starts_with("@@") {
        finish_hunk(file, hunk);
        *hunk = Some(DiffHunk {
            header: parse_hunk_header(line),
            new_range: parse_hunk_added_range(line),
            old_range: parse_hunk_removed_range(line),
            added_lines: Vec::new(),
            removed_lines: Vec::new(),
        });
        return;
    }
    append_hunk_line(line, hunk);
}

fn parse_hunk_header(line: &str) -> Option<String> {
    let (_, suffix) = line.strip_prefix("@@")?.split_once("@@")?;
    let header = suffix.trim();
    (!header.is_empty()).then(|| header.to_string())
}

fn finish_current_file(
    files: &mut Vec<ChangedFile>,
    current: &mut Option<ChangedFile>,
    trailing: Option<DiffHunk>,
) {
    if let Some(file) = current.take() {
        files.push(finalize_file(file, trailing));
    }
}

fn update_file_status(line: &str, file: &mut ChangedFile) -> bool {
    let status = if line.starts_with("new file mode ") {
        Some(ChangeStatus::Added)
    } else if line.starts_with("deleted file mode ") {
        Some(ChangeStatus::Deleted)
    } else if line.starts_with("rename from ") {
        Some(ChangeStatus::Renamed)
    } else {
        None
    };
    if let Some(status) = status {
        file.status = status;
        true
    } else {
        false
    }
}

fn update_file_path(line: &str, file: &mut ChangedFile) -> bool {
    if let Some(path) = line.strip_prefix("+++ ") {
        if let Some(path) = normalize_diff_path(path)
            && file.status != ChangeStatus::Deleted
        {
            file.path = PathBuf::from(path);
        }
        return true;
    }
    if let Some(path) = line.strip_prefix("--- ") {
        if let Some(path) = normalize_diff_path(path)
            && file.status == ChangeStatus::Deleted
        {
            file.path = PathBuf::from(path);
        }
        return true;
    }
    false
}

fn finish_hunk(file: &mut ChangedFile, hunk: &mut Option<DiffHunk>) {
    if let Some(done) = hunk.take() {
        file.hunks.push(done);
    }
}

fn append_hunk_line(line: &str, hunk: &mut Option<DiffHunk>) {
    let Some(active) = hunk.as_mut() else {
        return;
    };
    if let Some(added) = line.strip_prefix('+') {
        active.added_lines.push(added.to_string());
    } else if let Some(removed) = line.strip_prefix('-') {
        active.removed_lines.push(removed.to_string());
    }
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
