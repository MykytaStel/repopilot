//! Rust import resolution (`crate::`, `self::`, `super::`, `mod::`).

use super::probe;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub(super) fn resolve_rust(
    raw: &str,
    from_file: &Path,
    root: &Path,
    known_files: &HashSet<PathBuf>,
) -> Option<PathBuf> {
    if let Some(name) = raw.strip_prefix("mod::") {
        let dir = rust_current_module_dir(from_file, root);
        return probe(
            &[
                dir.join(format!("{name}.rs")),
                dir.join(name).join("mod.rs"),
            ],
            known_files,
        );
    }

    if let Some(rest) = raw.strip_prefix("crate::") {
        let src_root = root.join("src");
        return resolve_rust_module_path(&src_root, rest, known_files);
    }

    if let Some(rest) = raw.strip_prefix("self::") {
        let module_dir = rust_current_module_dir(from_file, root);
        return resolve_rust_module_path(&module_dir, rest, known_files);
    }

    if raw.starts_with("super::") {
        let mut remaining = raw;
        let mut base = rust_current_module_dir(from_file, root);
        while let Some(rest) = remaining.strip_prefix("super::") {
            base = base.parent().unwrap_or(root).to_path_buf();
            remaining = rest;
        }
        return resolve_rust_module_path(&base, remaining, known_files);
    }

    None
}

fn resolve_rust_module_path(
    base_dir: &Path,
    module_path: &str,
    known_files: &HashSet<PathBuf>,
) -> Option<PathBuf> {
    let segments = module_path
        .split("::")
        .filter(|segment| !segment.is_empty() && *segment != "self")
        .collect::<Vec<_>>();

    for end in (1..=segments.len()).rev() {
        let base = segments[..end]
            .iter()
            .fold(base_dir.to_path_buf(), |path, segment| path.join(segment));
        if let Some(path) = probe_rust_module_file(&base, known_files) {
            return Some(path);
        }
    }

    None
}

fn probe_rust_module_file(base: &Path, known_files: &HashSet<PathBuf>) -> Option<PathBuf> {
    probe(
        &[base.with_extension("rs"), base.join("mod.rs")],
        known_files,
    )
}

fn rust_current_module_dir(from_file: &Path, root: &Path) -> PathBuf {
    let src_root = root.join("src");
    let file_name = from_file.file_name().and_then(|name| name.to_str());

    match file_name {
        Some("lib.rs" | "main.rs" | "mod.rs") => from_file
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or(src_root),
        Some(_) => from_file.with_extension(""),
        None => src_root,
    }
}
