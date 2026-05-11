use crate::receipt::model::ReceiptGit;
use std::path::Path;
use std::process::Command;

pub fn collect_git_receipt(root: &Path) -> ReceiptGit {
    let git_root = if root.is_file() {
        root.parent().unwrap_or(root)
    } else {
        root
    };

    let is_git_repo = git_output(git_root, &["rev-parse", "--is-inside-work-tree"])
        .map(|value| value == "true")
        .unwrap_or(false);

    if !is_git_repo {
        return ReceiptGit {
            is_git_repo: false,
            branch: None,
            commit: None,
            dirty: false,
        };
    }

    let branch = git_output(git_root, &["rev-parse", "--abbrev-ref", "HEAD"])
        .filter(|branch| branch != "HEAD");

    let commit = git_output(git_root, &["rev-parse", "HEAD"]);

    let dirty = git_output_allow_empty(git_root, &["status", "--porcelain"])
        .map(|status| !status.trim().is_empty())
        .unwrap_or(false);

    ReceiptGit {
        is_git_repo,
        branch,
        commit,
        dirty,
    }
}

fn git_output(root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8(output.stdout).ok()?;
    let value = value.trim();

    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn git_output_allow_empty(root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    String::from_utf8(output.stdout).ok()
}
