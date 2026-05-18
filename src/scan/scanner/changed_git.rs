use crate::review::diff::{
    ChangedFile, DiffTarget, GitDiffError, load_changed_files, resolve_git_root,
};
use crate::scan::cache::CACHE_DIR;
use std::io;
use std::path::{Path, PathBuf};

pub(super) struct ChangedScope {
    pub(super) repo_root: PathBuf,
    pub(super) changed_files: Vec<ChangedFile>,
}

pub(super) fn collect_changed_scope(
    path: &Path,
    base_ref: Option<&str>,
) -> io::Result<ChangedScope> {
    let repo_root = resolve_git_root(path).map_err(diff_error_to_io)?;
    let pathspec = pathspec_for_scan_path(path, &repo_root);
    let target = match base_ref {
        Some(base) => DiffTarget::Refs { base, head: "HEAD" },
        None => DiffTarget::WorkingTree,
    };
    let changed_files = load_changed_files(&repo_root, target, pathspec.as_deref())
        .map_err(diff_error_to_io)?
        .into_iter()
        .filter(|changed_file| !is_cache_path(&changed_file.path))
        .collect::<Vec<_>>();

    Ok(ChangedScope {
        repo_root,
        changed_files,
    })
}

fn is_cache_path(path: &Path) -> bool {
    let path = path.to_string_lossy().replace('\\', "/");
    path == CACHE_DIR
        || path
            .strip_prefix(CACHE_DIR)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn pathspec_for_scan_path(scan_path: &Path, repo_root: &Path) -> Option<String> {
    let absolute = if scan_path.is_absolute() {
        scan_path.to_path_buf()
    } else {
        std::env::current_dir().ok()?.join(scan_path)
    };
    let absolute = absolute.canonicalize().unwrap_or(absolute);
    let relative = absolute.strip_prefix(repo_root).ok()?;
    if relative.as_os_str().is_empty() {
        None
    } else {
        Some(relative.to_string_lossy().replace('\\', "/"))
    }
}

fn diff_error_to_io(error: GitDiffError) -> io::Error {
    io::Error::other(error)
}
