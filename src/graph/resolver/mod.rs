//! Resolves raw import strings to concrete repository paths, per language.
//!
//! [`resolve_import`] dispatches on the importing file's extension to a
//! language-specific resolver submodule. Each submodule owns the import
//! semantics for one language family; the shared `probe` / [`normalize_path`]
//! helpers live here because every resolver depends on them.

mod go;
mod jvm;
mod python;
mod rust;
mod ts;

use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

/// Resolves a raw import string extracted from `from_file` to a concrete path
/// under `root`. Returns a path only when it exists in `known_files`.
pub fn resolve_import(
    raw_import: &str,
    from_file: &Path,
    root: &Path,
    known_files: &HashSet<PathBuf>,
) -> Option<PathBuf> {
    let ext = from_file.extension().and_then(|e| e.to_str()).unwrap_or("");

    match ext {
        "rs" => rust::resolve_rust(raw_import, from_file, root, known_files),
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" => {
            ts::resolve_ts(raw_import, from_file, root, known_files)
        }
        "py" => python::resolve_python(raw_import, from_file, root, known_files),
        "go" => go::resolve_go(raw_import, root, known_files),
        "java" => jvm::resolve_jvm(raw_import, root, known_files, &["java"]),
        "kt" | "kts" => jvm::resolve_jvm(raw_import, root, known_files, &["kt", "java"]),
        _ => None,
    }
}

/// Returns the first candidate that exists in `known_files`, after normalizing
/// `.`/`..` components. Shared by every language resolver.
fn probe(candidates: &[PathBuf], known_files: &HashSet<PathBuf>) -> Option<PathBuf> {
    for candidate in candidates {
        let normalized = normalize_path(candidate);
        if known_files.contains(&normalized) {
            return Some(normalized);
        }
    }
    None
}

/// Resolves `.` and `..` components without touching the filesystem.
pub(crate) fn normalize_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}
