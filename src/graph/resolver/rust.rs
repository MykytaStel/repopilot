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

    // `#[path = "..."] mod x;` and `include!("...")` both resolve relative to
    // the directory of the file that declares them.
    if let Some(rel) = raw.strip_prefix("relfile::") {
        let base = from_file
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| root.to_path_buf());
        return probe(&[base.join(rel)], known_files);
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

/// Enumerates Rust imports whose file target is fixed by the syntax itself.
///
/// A `mod name;` declaration can only be backed by `name.rs` or
/// `name/mod.rs` beside the declaring module. `#[path = "..."]` and
/// `include!("...")` are represented as `relfile::...` and name exactly one
/// path. `use crate::...` stays out of this set because it may refer to an
/// inline module or a re-export rather than a file.
pub(super) fn definitive_local_candidates(
    raw: &str,
    from_file: &Path,
    root: &Path,
) -> Option<Vec<PathBuf>> {
    let candidates = if let Some(name) = raw.strip_prefix("mod::") {
        if !is_module_name(name) {
            return None;
        }
        let dir = rust_current_module_dir(from_file, root);
        vec![
            dir.join(format!("{name}.rs")),
            dir.join(name).join("mod.rs"),
        ]
    } else {
        let rel = raw.strip_prefix("relfile::")?;
        let path = Path::new(rel);
        if rel.is_empty() || path.is_absolute() || rel.contains('\0') {
            return None;
        }
        let base = from_file.parent().unwrap_or(root);
        vec![base.join(path)]
    };

    let root = super::normalize_path(root);
    let candidates = candidates
        .into_iter()
        .map(|candidate| super::normalize_path(&candidate))
        .collect::<Vec<_>>();
    (candidates
        .iter()
        .all(|candidate| candidate.starts_with(&root)))
    .then_some(candidates)
}

fn is_module_name(name: &str) -> bool {
    let identifier = name.strip_prefix("r#").unwrap_or(name);
    !identifier.is_empty()
        && name != "self"
        && name != "super"
        && name != "crate"
        && is_rust_identifier(identifier)
}

fn is_rust_identifier(identifier: &str) -> bool {
    let mut chars = identifier.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_alphabetic()) && chars.all(|ch| ch == '_' || ch.is_alphanumeric())
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
