use crate::scan::config::ScanConfig;
use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

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
    let matcher = PathMatcher::new(config, repopilotignore_path.as_deref())?;
    let skipped_repopilotignore = Arc::new(AtomicUsize::new(0));
    let collected =
        collect_paths_with_matcher(path, matcher, Arc::clone(&skipped_repopilotignore))?;

    Ok(CollectedPaths {
        file_paths: collected.file_paths,
        directories_count: collected.directories_count,
        files_skipped_repopilotignore: skipped_repopilotignore.load(Ordering::Relaxed),
        repopilotignore_path,
    })
}

fn collect_paths_with_matcher(
    path: &Path,
    matcher: PathMatcher,
    skipped_repopilotignore: Arc<AtomicUsize>,
) -> io::Result<CollectedPaths> {
    let mut file_paths = Vec::new();
    let mut directories_count = 0usize;

    let skipped_clone = Arc::clone(&skipped_repopilotignore);
    for result in build_walker(path, matcher, skipped_clone) {
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

    let files_skipped_repopilotignore = skipped_repopilotignore.load(Ordering::Relaxed);

    Ok(CollectedPaths {
        file_paths,
        directories_count,
        files_skipped_repopilotignore,
        repopilotignore_path: None,
    })
}

fn build_walker(
    path: &Path,
    matcher: PathMatcher,
    skipped_repopilotignore: Arc<AtomicUsize>,
) -> ignore::Walk {
    let root = path.to_path_buf();

    let mut builder = WalkBuilder::new(path);

    builder
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true);

    builder
        .filter_entry(move |entry| {
            if entry.path() == root {
                return true;
            }
            match matcher.match_path(entry.path(), &root) {
                IgnoreMatch::BuiltInOrConfig => false,
                IgnoreMatch::RepopilotIgnore => {
                    if entry
                        .file_type()
                        .is_some_and(|file_type| file_type.is_file())
                    {
                        skipped_repopilotignore.fetch_add(1, Ordering::Relaxed);
                    }
                    false
                }
                IgnoreMatch::None => true,
            }
        })
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

fn is_repopilot_control_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| matches!(name, REPOPILOT_IGNORE_FILENAME | "repopilot.toml"))
        .unwrap_or(false)
}

#[derive(Clone)]
struct PathMatcher {
    built_in_or_config: GlobSet,
    repopilotignore: GlobSet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IgnoreMatch {
    None,
    BuiltInOrConfig,
    RepopilotIgnore,
}

impl PathMatcher {
    fn new(config: &ScanConfig, repopilotignore_path: Option<&Path>) -> io::Result<Self> {
        let mut built_in_or_config = GlobSetBuilder::new();
        for pattern in config
            .ignored_paths
            .iter()
            .chain(config.exclude_patterns.iter())
        {
            add_path_pattern(&mut built_in_or_config, pattern)?;
        }

        let mut repopilotignore = GlobSetBuilder::new();
        if let Some(path) = repopilotignore_path {
            for pattern in read_repopilotignore_patterns(path)? {
                add_path_pattern(&mut repopilotignore, &pattern)?;
            }
        }

        Ok(Self {
            built_in_or_config: built_in_or_config.build().map_err(io::Error::other)?,
            repopilotignore: repopilotignore.build().map_err(io::Error::other)?,
        })
    }

    fn match_path(&self, path: &Path, root: &Path) -> IgnoreMatch {
        let relative = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();

        if self.built_in_or_config.is_match(relative.as_str())
            || self.built_in_or_config.is_match(name)
        {
            return IgnoreMatch::BuiltInOrConfig;
        }

        if self.repopilotignore.is_match(relative.as_str()) || self.repopilotignore.is_match(name) {
            return IgnoreMatch::RepopilotIgnore;
        }

        IgnoreMatch::None
    }
}

fn read_repopilotignore_patterns(path: &Path) -> io::Result<Vec<String>> {
    let content = fs::read_to_string(path)?;
    Ok(content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect())
}

fn add_path_pattern(builder: &mut GlobSetBuilder, raw_pattern: &str) -> io::Result<()> {
    let pattern = raw_pattern.trim().trim_matches('/');
    if pattern.is_empty() {
        return Ok(());
    }

    add_glob(builder, pattern)?;
    if !pattern.contains('/') {
        add_glob(builder, &format!("**/{pattern}"))?;
        add_glob(builder, &format!("{pattern}/**"))?;
        add_glob(builder, &format!("**/{pattern}/**"))?;
    } else if !pattern.ends_with("/**") {
        add_glob(builder, &format!("{pattern}/**"))?;
    }

    Ok(())
}

fn add_glob(builder: &mut GlobSetBuilder, pattern: &str) -> io::Result<()> {
    let glob = Glob::new(pattern).map_err(io::Error::other)?;
    builder.add(glob);
    Ok(())
}
