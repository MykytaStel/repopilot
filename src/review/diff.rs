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

fn git_diff_against_head(repo_root: &Path, pathspec: Option<&str>) -> Result<String, GitDiffError> {
    let mut args = vec!["diff", "--unified=0", "--no-ext-diff", "HEAD", "--"];
    if let Some(pathspec) = pathspec {
        args.push(pathspec);
    }

    git_output(repo_root, &args, "git diff --unified=0 --no-ext-diff HEAD")
}

fn git_diff_between_refs(
    repo_root: &Path,
    base: &str,
    head: &str,
    pathspec: Option<&str>,
) -> Result<String, GitDiffError> {
    let range = format!("{base}...{head}");
    let mut args = vec!["diff", "--unified=0", "--no-ext-diff", range.as_str(), "--"];
    if let Some(pathspec) = pathspec {
        args.push(pathspec);
    }

    git_output(
        repo_root,
        &args,
        &format!("git diff --unified=0 --no-ext-diff {range}"),
    )
}

fn load_untracked_files(
    repo_root: &Path,
    pathspec: Option<&str>,
) -> Result<Vec<ChangedFile>, GitDiffError> {
    let mut args = vec!["ls-files", "--others", "--exclude-standard", "-z", "--"];
    if let Some(pathspec) = pathspec {
        args.push(pathspec);
    }

    let output = git_output(repo_root, &args, "git ls-files --others --exclude-standard")?;

    output
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(|path| {
            let line_count = fs::read_to_string(repo_root.join(path))
                .map(|content| content.lines().count())
                .unwrap_or(0);
            let ranges = if line_count == 0 {
                Vec::new()
            } else {
                vec![ChangedRange {
                    start: 1,
                    end: line_count,
                }]
            };

            Ok(ChangedFile {
                path: PathBuf::from(path),
                status: ChangeStatus::Untracked,
                ranges,
            })
        })
        .collect()
}

fn is_repopilot_internal_path(path: &Path) -> bool {
    let normalized = path.to_string_lossy().replace('\\', "/");
    normalized == ".repopilot" || normalized.starts_with(".repopilot/")
}

fn git_output(cwd: &Path, args: &[&str], command_label: &str) -> Result<String, GitDiffError> {
    let output = Command::new("git").args(args).current_dir(cwd).output()?;

    if !output.status.success() {
        return Err(GitDiffError::GitCommandFailed {
            command: command_label.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn parse_diff_git_line(line: &str) -> Option<(String, String)> {
    let rest = line.strip_prefix("diff --git ")?;
    let mut parts = rest.split_whitespace();
    let old_path = normalize_diff_path(parts.next()?)?;
    let new_path = normalize_diff_path(parts.next()?)?;

    Some((old_path, new_path))
}

fn normalize_diff_path(path: &str) -> Option<String> {
    let path = path.trim();

    if path == "/dev/null" {
        return None;
    }

    let path = path
        .strip_prefix("a/")
        .or_else(|| path.strip_prefix("b/"))
        .unwrap_or(path);

    Some(path.trim_matches('"').replace('\\', "/"))
}

fn parse_hunk_added_range(line: &str) -> Option<ChangedRange> {
    let range = line.split_once(" +")?.1.split_once(" @@")?.0;
    let mut parts = range.split(',');
    let start = parts.next()?.parse::<usize>().ok()?;
    let count = parts
        .next()
        .and_then(|count| count.parse::<usize>().ok())
        .unwrap_or(1);

    if count == 0 {
        return None;
    }

    Some(ChangedRange {
        start,
        end: start + count - 1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repopilot_internal_paths_are_excluded_from_review_scope() {
        assert!(is_repopilot_internal_path(Path::new(
            ".repopilot/cache/repo_context.json"
        )));
        assert!(is_repopilot_internal_path(Path::new(
            r".repopilot\cache\repo_context.json"
        )));
        assert!(!is_repopilot_internal_path(Path::new("src/lib.rs")));
    }
}
