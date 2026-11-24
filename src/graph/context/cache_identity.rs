use crate::path_security::RootConfinement;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
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
/// Include a bounded content fingerprint so unchanged dirty paths outside a
/// scoped review can be reused without confusing them with subsequent edits.
/// An unavailable or non-UTF-8 inventory must not authorize a cache patch.
pub(super) fn dirty_inputs(root: &Path) -> Option<BTreeMap<PathBuf, String>> {
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
    let confinement = RootConfinement::new(root).ok()?;
    let mut paths = BTreeMap::new();
    for entry in text.split_terminator('\0') {
        let path = PathBuf::from(entry.get(3..)?);
        if entry.as_bytes().get(2) != Some(&b' ') || path.as_os_str().is_empty() {
            return None;
        }
        if path.starts_with(".repopilot/cache") {
            continue;
        }
        if !path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
        {
            return None;
        }
        let fingerprint = dirty_input_fingerprint(&confinement, &path, &entry.as_bytes()[..2])?;
        paths.insert(path, fingerprint);
    }
    Some(paths)
}

const MAX_DIRTY_INPUT_BYTES: u64 = 8 * 1024 * 1024;

fn dirty_input_fingerprint(
    confinement: &RootConfinement,
    relative: &Path,
    status: &[u8],
) -> Option<String> {
    let candidate = confinement.root().join(relative);
    let mut hasher = Sha256::new();
    hasher.update(status);
    match fs::symlink_metadata(&candidate) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            hasher.update(b"missing");
        }
        Ok(metadata) if metadata.is_file() && metadata.len() <= MAX_DIRTY_INPUT_BYTES => {
            let resolved = confinement.resolve_existing(relative, "dirty input").ok()?;
            let mut file = fs::File::open(resolved)
                .ok()?
                .take(MAX_DIRTY_INPUT_BYTES + 1);
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes).ok()?;
            if bytes.len() as u64 > MAX_DIRTY_INPUT_BYTES {
                return None;
            }
            hasher.update(b"file");
            hasher.update(&bytes);
        }
        // Symlinks, submodules, unreadable and oversized inputs cannot prove
        // freshness. Fall back instead of following links or hashing a prefix.
        _ => return None,
    }
    Some(
        hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    )
}
