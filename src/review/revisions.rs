//! Resolve the Git commits behind a review target so proof provenance can name
//! the exact revisions it assessed instead of reporting them as unavailable.

use std::path::Path;
use std::process::Command;

use crate::review::diff::OwnedDiffTarget;
use crate::review::model::ReviewRevisions;

pub fn resolve_revisions(repo_root: &Path, target: &OwnedDiffTarget) -> ReviewRevisions {
    match target {
        OwnedDiffTarget::WorkingTree => ReviewRevisions {
            base_commit: resolve_commit(repo_root, "HEAD"),
            head_commit: None,
            head_is_working_tree: true,
        },
        OwnedDiffTarget::SinceRef { base } => ReviewRevisions {
            base_commit: resolve_commit(repo_root, base),
            head_commit: None,
            head_is_working_tree: true,
        },
        OwnedDiffTarget::Refs { base, head } => ReviewRevisions {
            base_commit: resolve_commit(repo_root, base),
            head_commit: resolve_commit(repo_root, head),
            head_is_working_tree: false,
        },
    }
}

fn resolve_commit(repo_root: &Path, reference: &str) -> Option<String> {
    if reference.is_empty() || reference.starts_with('-') {
        return None;
    }
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["rev-parse", "--verify", "--quiet"])
        .arg(format!("{reference}^{{commit}}"))
        .output()
        .ok()
        .filter(|output| output.status.success())?;
    let commit = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (commit.len() >= 40 && commit.chars().all(|c| c.is_ascii_hexdigit())).then_some(commit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_like_and_empty_refs_are_never_passed_to_git() {
        let root = Path::new(".");
        assert_eq!(resolve_commit(root, ""), None);
        assert_eq!(resolve_commit(root, "--output=/tmp/x"), None);
    }

    #[test]
    fn unknown_refs_stay_unresolved() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        assert_eq!(
            resolve_commit(root, "refs/heads/repopilot-no-such-branch-xyz"),
            None
        );
    }
}
