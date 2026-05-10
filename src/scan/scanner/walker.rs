use crate::scan::config::ScanConfig;
use ignore::WalkBuilder;
use std::io;
use std::path::{Path, PathBuf};

pub(super) fn collect_paths(path: &Path, config: &ScanConfig) -> io::Result<(Vec<PathBuf>, usize)> {
    let mut file_paths = Vec::new();
    let mut dirs_count = 0usize;

    for result in build_walker(path, config) {
        let entry = result.map_err(io::Error::other)?;
        let entry_path = entry.path();

        if entry_path == path {
            continue;
        }

        let Some(file_type) = entry.file_type() else {
            continue;
        };

        if file_type.is_dir() {
            dirs_count += 1;
        } else if file_type.is_file() {
            file_paths.push(entry_path.to_path_buf());
        }
    }

    Ok((file_paths, dirs_count))
}

pub(super) fn build_walker(path: &Path, config: &ScanConfig) -> ignore::Walk {
    let root = path.to_path_buf();
    let mut ignored_paths = config.ignored_paths.clone();
    ignored_paths.extend(
        config
            .exclude_patterns
            .iter()
            .map(|pattern| pattern.trim_end_matches("/**").to_string()),
    );
    WalkBuilder::new(path)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .add_custom_ignore_filename(".repopilotignore")
        .filter_entry(move |entry| !is_ignored_path(entry.path(), &root, &ignored_paths))
        .build()
}

fn is_ignored_path(path: &Path, root: &Path, ignored_paths: &[String]) -> bool {
    if path == root {
        return false;
    }

    ignored_paths.iter().any(|ignored_path| {
        let ignored_path = ignored_path.trim_matches('/');

        if ignored_path.is_empty() {
            return false;
        }

        path.strip_prefix(root)
            .ok()
            .and_then(|relative_path| relative_path.to_str())
            .map(|relative_path| relative_path == ignored_path)
            .unwrap_or(false)
            || path
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name == ignored_path)
                .unwrap_or(false)
    })
}
