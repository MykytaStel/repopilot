//! The exact working tree at snapshot time, as a Git commit object.
//!
//! When the tree is dirty, `HEAD` alone would attribute edits that predate
//! the session to it. The baseline stages the working tree into a temporary
//! index (seeded from the real one, so unchanged files are not rehashed),
//! writes it as a tree, wraps it in a commit whose parent is `HEAD`, and pins
//! it under `refs/repopilot/snapshot` so garbage collection keeps it. The
//! user's index, working tree, and branches are untouched.

use super::SnapshotError;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(super) const BASELINE_REF: &str = "refs/repopilot/snapshot";
const IDENTITY: [(&str, &str); 4] = [
    ("GIT_AUTHOR_NAME", "RepoPilot"),
    ("GIT_AUTHOR_EMAIL", "repopilot@localhost"),
    ("GIT_COMMITTER_NAME", "RepoPilot"),
    ("GIT_COMMITTER_EMAIL", "repopilot@localhost"),
];

pub(super) fn record(repo_root: &Path, head: &str) -> Result<String, SnapshotError> {
    let index = PathBuf::from(git(
        repo_root,
        &["rev-parse", "--git-path", "repopilot-snapshot.index"],
        &[],
    )?);
    let index = if index.is_absolute() {
        index
    } else {
        repo_root.join(index)
    };
    let real_index = PathBuf::from(git(repo_root, &["rev-parse", "--git-path", "index"], &[])?);
    let real_index = if real_index.is_absolute() {
        real_index
    } else {
        repo_root.join(real_index)
    };
    if real_index.is_file() {
        fs::copy(&real_index, &index).map_err(|source| SnapshotError::Io {
            path: index.clone(),
            source,
        })?;
    }
    let index_env = [("GIT_INDEX_FILE", index.to_string_lossy().to_string())];
    let result = (|| {
        git(repo_root, &["add", "-A", "--", "."], &index_env)?;
        let tree = git(repo_root, &["write-tree"], &index_env)?;
        let commit = git(
            repo_root,
            &[
                "commit-tree",
                &tree,
                "-p",
                head,
                "-m",
                "repopilot snapshot baseline",
            ],
            &[],
        )?;
        git(repo_root, &["update-ref", BASELINE_REF, &commit], &[])?;
        Ok(commit)
    })();
    let _ = fs::remove_file(&index);
    result
}

fn git(repo_root: &Path, args: &[&str], env: &[(&str, String)]) -> Result<String, SnapshotError> {
    let mut command = Command::new("git");
    command.args(args).current_dir(repo_root);
    for (key, value) in IDENTITY {
        if std::env::var_os(key).is_none() {
            command.env(key, value);
        }
    }
    for (key, value) in env {
        command.env(key, value);
    }
    let output = command.output().map_err(|source| SnapshotError::Io {
        path: repo_root.to_path_buf(),
        source,
    })?;
    if !output.status.success() {
        return Err(SnapshotError::Baseline {
            command: format!("git {}", args.join(" ")),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
