use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(super) fn stable_hash_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(super) fn resolver_input_fingerprint(root: &Path) -> String {
    let content = crate::graph::resolver::effective_tsconfig_content(root);
    stable_hash_hex(
        serde_json::to_string(&content)
            .expect("config content serializes")
            .as_bytes(),
    )
}

/// Capture the entire worktree's dirty inventory, not the current review's
/// pathspec. Disable rename detection so both old and new paths are present.
/// An unavailable or non-UTF-8 inventory must not authorize a cache patch.
pub(super) fn dirty_paths(root: &Path) -> Option<BTreeSet<PathBuf>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
            "--no-renames",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let mut paths = BTreeSet::new();
    for entry in text.split_terminator('\0') {
        let path = PathBuf::from(entry.get(3..)?);
        if entry.as_bytes().get(2) != Some(&b' ') || path.as_os_str().is_empty() {
            return None;
        }
        if path.starts_with(".repopilot/cache") {
            continue;
        }
        paths.insert(path);
    }
    Some(paths)
}
