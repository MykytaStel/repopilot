//! Shared path normalization for the review layer.
//!
//! Lives in its own leaf module so both `review` (blast radius, pathspec) and
//! `review::signals::composites` can use it without creating an import cycle
//! between the parent module and its `signals` child.

use crate::baseline::key::normalized_relative_path;
use std::path::{Path, PathBuf};

/// Normalize `path` to a repo-root-relative path, canonicalizing where possible
/// so diff paths, coupling-graph paths, and signal paths compare equal.
pub(crate) fn normalized_review_path(path: &Path, repo_root: &Path) -> PathBuf {
    let repo_root = repo_root
        .canonicalize()
        .unwrap_or_else(|_| repo_root.to_path_buf());

    let path = if path.is_absolute() {
        path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
    } else {
        let repo_path = repo_root.join(path);
        repo_path.canonicalize().unwrap_or(repo_path)
    };

    PathBuf::from(normalized_relative_path(&path, &repo_root))
}
