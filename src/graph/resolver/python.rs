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

        return resolve_python_module_from_base(dir, module, known_files);
    }

    for base in [root.to_path_buf(), root.join("src")] {
        if let Some(path) = resolve_python_module_from_base(&base, raw, known_files) {
            return Some(path);
        }
    }

    None
}

fn resolve_python_module_from_base(
    base_dir: &Path,
    module: &str,
    known_files: &HashSet<PathBuf>,
) -> Option<PathBuf> {
    if module.is_empty() {
        return probe(&[base_dir.join("__init__.py")], known_files);
    }

    let segments = module
        .split('.')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();

    for end in (1..=segments.len()).rev() {
        let base = segments[..end]
            .iter()
            .fold(base_dir.to_path_buf(), |path, segment| path.join(segment));
        if let Some(path) = probe(
            &[base.with_extension("py"), base.join("__init__.py")],
            known_files,
        ) {
            return Some(path);
        }
    }

    None
}
