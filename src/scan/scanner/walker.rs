use crate::scan::config::ScanConfig;
use ignore::WalkBuilder;
use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};

const REPOPILOT_IGNORE_FILENAME: &str = ".repopilotignore";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CollectedPaths {
    pub(super) file_paths: Vec<PathBuf>,
    pub(super) directories_count: usize,
    pub(super) files_skipped_repopilotignore: usize,
    pub(super) repopilotignore_path: Option<PathBuf>,
}

pub(super) fn collect_paths(path: &Path, config: &ScanConfig) -> io::Result<CollectedPaths> {
    let repopilotignore_path = find_repopilotignore(path);

    // Build the combined ignored-paths set once and reuse for both walks.
    let ignored = build_ignored_set(config);

    let with_repopilotignore = collect_paths_with_custom_ignore(path, &ignored, true)?;

    let files_skipped_repopilotignore = if repopilotignore_path.is_some() {
        let without_repopilotignore_count = count_files_with_custom_ignore(path, &ignored, false)?;

        without_repopilotignore_count.saturating_sub(with_repopilotignore.file_paths.len())
    } else {
        0
    };

    Ok(CollectedPaths {
        file_paths: with_repopilotignore.file_paths,
        directories_count: with_repopilotignore.directories_count,
        files_skipped_repopilotignore,
        repopilotignore_path,
    })
}

fn collect_paths_with_custom_ignore(
    path: &Path,
    ignored: &HashSet<String>,
    use_repopilotignore: bool,
) -> io::Result<CollectedPaths> {
    let mut file_paths = Vec::new();
    let mut directories_count = 0usize;

    for result in build_walker(path, ignored, use_repopilotignore) {
        let entry = result.map_err(io::Error::other)?;
        let entry_path = entry.path();

        if entry_path == path {
            continue;
        }

        let Some(file_type) = entry.file_type() else {
            continue;
        };

        if file_type.is_dir() {
            directories_count += 1;
        } else if file_type.is_file() && !is_repopilot_control_file(entry_path) {
            file_paths.push(entry_path.to_path_buf());
        }
    }

    Ok(CollectedPaths {
        file_paths,
        directories_count,
        files_skipped_repopilotignore: 0,
        repopilotignore_path: None,
    })
}

fn count_files_with_custom_ignore(
    path: &Path,
    ignored: &HashSet<String>,
    use_repopilotignore: bool,
) -> io::Result<usize> {
    let mut files_count = 0usize;

    for result in build_walker(path, ignored, use_repopilotignore) {
        let entry = result.map_err(io::Error::other)?;
        let entry_path = entry.path();

        if entry_path == path {
            continue;
        }

        let Some(file_type) = entry.file_type() else {
            continue;
        };

        if file_type.is_file() && !is_repopilot_control_file(entry_path) {
            files_count += 1;
        }
    }

    Ok(files_count)
}

fn build_ignored_set(config: &ScanConfig) -> HashSet<String> {
    let mut ignored: HashSet<String> = config
        .ignored_paths
        .iter()
        .map(|p| p.trim_matches('/').to_string())
        .filter(|p| !p.is_empty())
        .collect();

    ignored.extend(
        config
            .exclude_patterns
            .iter()
            .map(|p| p.trim_end_matches("/**").trim_matches('/').to_string())
            .filter(|p| !p.is_empty()),
    );

    ignored
}

fn build_walker(path: &Path, ignored: &HashSet<String>, use_repopilotignore: bool) -> ignore::Walk {
    let root = path.to_path_buf();
    let ignored = ignored.clone();

    let mut builder = WalkBuilder::new(path);

    builder
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true);

    if use_repopilotignore {
        builder.add_custom_ignore_filename(REPOPILOT_IGNORE_FILENAME);
    }

    builder
        .filter_entry(move |entry| !is_ignored_path(entry.path(), &root, &ignored))
        .build()
}

fn find_repopilotignore(path: &Path) -> Option<PathBuf> {
    let root = if path.is_file() {
        path.parent().unwrap_or(path)
    } else {
        path
    };

    let candidate = root.join(REPOPILOT_IGNORE_FILENAME);

    if candidate.is_file() {
        Some(candidate)
    } else {
        None
    }
}

fn is_ignored_path(path: &Path, root: &Path, ignored: &HashSet<String>) -> bool {
    if path == root || ignored.is_empty() {
        return false;
    }

    if let Some(relative) = path.strip_prefix(root).ok().and_then(|p| p.to_str()) {
        if ignored.contains(relative) {
            return true;
        }
    }

    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        if ignored.contains(name) {
            return true;
        }
    }

    false
}

fn is_repopilot_control_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| matches!(name, REPOPILOT_IGNORE_FILENAME | "repopilot.toml"))
        .unwrap_or(false)
}
