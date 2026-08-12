//! Python import resolution (relative `.`/`..` imports and absolute packages).

use super::probe;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub(super) fn resolve_python(
    raw: &str,
    from_file: &Path,
    root: &Path,
    known_files: &HashSet<PathBuf>,
) -> Option<PathBuf> {
    if raw.starts_with('.') {
        let dots = raw.chars().take_while(|c| *c == '.').count();
        let module = &raw[dots..];

        let mut dir = from_file.parent()?;
        for _ in 0..dots.saturating_sub(1) {
            dir = dir.parent().unwrap_or(dir);
        }

        return probe(&python_module_candidates(dir, module), known_files);
    }

    for base in [root.to_path_buf(), root.join("src")] {
        if let Some(path) = probe(&python_module_candidates(&base, raw), known_files) {
            return Some(path);
        }
    }

    None
}

fn python_module_candidates(base_dir: &Path, module: &str) -> Vec<PathBuf> {
    if module.is_empty() {
        return vec![base_dir.join("__init__.py")];
    }

    let segments = module
        .split('.')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();

    let mut candidates = Vec::new();
    for end in (1..=segments.len()).rev() {
        let base = segments[..end]
            .iter()
            .fold(base_dir.to_path_buf(), |path, segment| path.join(segment));
        candidates.push(base.with_extension("py"));
        candidates.push(base.join("__init__.py"));
    }
    candidates
}

pub(super) fn definitive_relative_candidates(raw: &str, from_file: &Path) -> Option<Vec<PathBuf>> {
    let dots = raw.chars().take_while(|value| *value == '.').count();
    let module = raw.get(dots..)?;
    if dots == 0 || module.is_empty() {
        return None;
    }
    let mut dir = from_file.parent()?;
    for _ in 0..dots.saturating_sub(1) {
        dir = dir.parent()?;
    }
    Some(python_module_candidates(dir, module))
}
