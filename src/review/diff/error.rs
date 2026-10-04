//! Errors from loading a Git diff. The two a first run usually hits, a folder
//! outside Git and a repository without commits, get plain messages that say
//! what to run instead.

use std::error::Error;
use std::fmt;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub enum GitDiffError {
    PathNotFound(PathBuf),
    GitNotFound,
    /// The path is not inside a Git working tree.
    NotARepository(PathBuf),
    /// The repository has no commits, so there is no `HEAD` to diff against.
    NoCommits,
    GitCommandFailed {
        command: String,
        stderr: String,
    },
    Io(io::Error),
}

impl fmt::Display for GitDiffError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitDiffError::PathNotFound(path) => {
                write!(formatter, "path does not exist: {}", path.display())
            }
            GitDiffError::GitNotFound => write!(
                formatter,
                "git executable was not found; `repopilot review` requires git"
            ),
            GitDiffError::NotARepository(path) => write!(
                formatter,
                "{} is not inside a Git repository, and reviewing changes needs Git. To audit the folder without Git, run `repopilot scan {}`.",
                path.display(),
                path.display()
            ),
            GitDiffError::NoCommits => write!(
                formatter,
                "this repository has no commits yet, so there is no HEAD to compare against. Commit once, then review later changes, or run `repopilot scan .` for a full audit."
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
