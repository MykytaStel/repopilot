//! Untracked files against a snapshot baseline.
//!
//! A ref-vs-worktree `git diff` only sees tracked files, so worktree-backed
//! reviews list untracked files separately as new. When the base is a snapshot
//! baseline that recorded the working tree, an untracked file may already be in
//! it: unchanged since the snapshot (not part of this change) or edited since
//! (a modification, not an addition). Blob hashes decide which, without
//! writing anything to the repository. The same files also appear in the
//! tracked diff as deletions, because the index does not hold them; those
//! phantom deletions are dropped.

use super::{ChangeStatus, ChangedFile, GitDiffError, git_output};
use std::collections::HashMap;
use std::path::Path;

const PATHS_PER_CALL: usize = 500;

pub(super) fn reconcile_untracked_with_base(
    repo_root: &Path,
    base: &str,
    files: &mut Vec<ChangedFile>,
) -> Result<(), GitDiffError> {
    let untracked: Vec<String> = files
        .iter()
        .filter(|file| file.status == ChangeStatus::Untracked)
        .map(ChangedFile::path_string)
        .collect();
    if untracked.is_empty() {
        return Ok(());
    }
    let in_base = base_blobs(repo_root, base, &untracked)?;
    if in_base.is_empty() {
        return Ok(());
    }
    let present: Vec<String> = untracked
        .into_iter()
        .filter(|path| in_base.contains_key(path))
        .collect();
    let current = worktree_blobs(repo_root, &present)?;

    // `git diff <baseline>` reports a path the baseline holds but the index
    // does not as deleted, even though it is still on disk, untracked.
    files.retain(|file| {
        !(file.status == ChangeStatus::Deleted && current.contains_key(&file.path_string()))
    });
    files.retain_mut(|file| {
        if file.status != ChangeStatus::Untracked {
            return true;
        }
        let path = file.path_string();
        match (in_base.get(&path), current.get(&path)) {
            (Some(base_blob), Some(current_blob)) if base_blob == current_blob => false,
            (Some(_), _) => {
                file.status = ChangeStatus::Modified;
                true
            }
            _ => true,
        }
    });
    Ok(())
}

/// Blob ids of `paths` in the base tree (`git ls-tree`), for those it holds.
fn base_blobs(
    repo_root: &Path,
    base: &str,
    paths: &[String],
) -> Result<HashMap<String, String>, GitDiffError> {
    let mut blobs = HashMap::new();
    for chunk in paths.chunks(PATHS_PER_CALL) {
        let mut args = vec!["ls-tree", "-z", base, "--"];
        args.extend(chunk.iter().map(String::as_str));
        let output = git_output(repo_root, &args, "git ls-tree")?;
        for entry in output.split('\0').filter(|entry| !entry.is_empty()) {
            // `<mode> blob <sha>\t<path>`
            let Some((meta, path)) = entry.split_once('\t') else {
                continue;
            };
            let mut fields = meta.split_whitespace();
            if let (Some(_mode), Some("blob"), Some(sha)) =
                (fields.next(), fields.next(), fields.next())
            {
                blobs.insert(path.to_string(), sha.to_string());
            }
        }
    }
    Ok(blobs)
}

/// Blob ids the working-tree files would have (`git hash-object`, no `-w`).
fn worktree_blobs(
    repo_root: &Path,
    paths: &[String],
) -> Result<HashMap<String, String>, GitDiffError> {
    let mut blobs = HashMap::new();
    for chunk in paths.chunks(PATHS_PER_CALL) {
        let mut args = vec!["hash-object", "--"];
        args.extend(chunk.iter().map(String::as_str));
        let output = git_output(repo_root, &args, "git hash-object")?;
        for (path, sha) in chunk.iter().zip(output.lines()) {
            blobs.insert(path.clone(), sha.trim().to_string());
        }
    }
    Ok(blobs)
}
