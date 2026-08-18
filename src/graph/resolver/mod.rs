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

/// Enumerates every local file candidate only for language forms whose lookup
/// semantics are bounded enough for a missing-target claim.
pub(crate) fn definitive_local_candidates(
    raw_import: &str,
    from_file: &Path,
    root: &Path,
) -> Option<Vec<PathBuf>> {
    let root = normalize_path(root);
    let normalized_source = normalize_path(from_file);
    let source = if from_file.is_absolute() || normalized_source.starts_with(&root) {
        normalized_source
    } else {
        normalize_path(&root.join(normalized_source))
    };
    let ext = source.extension().and_then(|value| value.to_str())?;
    let candidates = match ext {
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" => {
            ts::definitive_relative_candidates(raw_import, &source)?
        }
        "py" => python::definitive_relative_candidates(raw_import, &source)?,
        _ => return None,
    };
    let candidates = candidates
        .into_iter()
        .map(|candidate| normalize_path(&candidate))
        .collect::<Vec<_>>();

    (!candidates.is_empty() && candidates.iter().all(|path| path.starts_with(&root)))
        .then_some(candidates)
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

#[cfg(test)]
mod definitive_candidates_tests {
    use super::*;

    #[test]
    fn explicit_typescript_relative_import_has_bounded_local_candidates() {
        let candidates = definitive_local_candidates(
            "./missing.ts",
            Path::new("/repo/src/app.ts"),
            Path::new("/repo"),
        )
        .expect("explicit supported TypeScript import should be definitive");

        assert_eq!(
            candidates,
            vec![
                PathBuf::from("/repo/src/missing.ts"),
                PathBuf::from("/repo/src/missing.tsx"),
                PathBuf::from("/repo/src/missing.js"),
                PathBuf::from("/repo/src/missing.jsx"),
            ]
        );
    }

    #[test]
    fn relative_source_already_rooted_under_relative_scan_path_is_not_prefixed_twice() {
        let candidates = definitive_local_candidates(
            "./missing.ts",
            Path::new("project/src/app.ts"),
            Path::new("project"),
        )
        .expect("relative rooted source should be definitive");

        assert_eq!(candidates[0], PathBuf::from("project/src/missing.ts"));
    }

    #[test]
    fn extensionless_typescript_import_is_not_claimed_as_definitive() {
        assert_eq!(
            definitive_local_candidates(
                "./generated-client",
                Path::new("/repo/src/app.ts"),
                Path::new("/repo"),
            ),
            None
        );
    }

    #[test]
    fn explicit_python_relative_module_has_module_and_package_candidates() {
        let candidates = definitive_local_candidates(
            ".missing",
            Path::new("/repo/pkg/app.py"),
            Path::new("/repo"),
        )
        .expect("explicit Python relative module should be definitive");

        assert_eq!(
            candidates,
            vec![
                PathBuf::from("/repo/pkg/missing.py"),
                PathBuf::from("/repo/pkg/missing/__init__.py"),
            ]
        );
    }

    #[test]
    fn candidates_that_escape_the_repository_are_not_definitive() {
        assert_eq!(
            definitive_local_candidates(
                "../../outside.ts",
                Path::new("/repo/src/app.ts"),
                Path::new("/repo"),
            ),
            None
        );
    }

    #[test]
    fn rust_module_semantics_remain_limited_in_first_slice() {
        assert_eq!(
            definitive_local_candidates(
                "mod::missing",
                Path::new("/repo/src/lib.rs"),
                Path::new("/repo"),
            ),
            None
        );
    }
}
