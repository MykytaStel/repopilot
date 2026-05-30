//! JVM (Java / Kotlin) import resolution from fully-qualified class names.

use super::{normalize_path, probe};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Resolves a fully-qualified JVM class name (`com.example.Foo`) to a source
/// file. Tries the standard Maven/Gradle source-root layout first, then falls
/// back to bare `src/`.
pub(super) fn resolve_jvm(
    raw: &str,
    root: &Path,
    known_files: &HashSet<PathBuf>,
    extensions: &[&str],
) -> Option<PathBuf> {
    let rel = raw.replace('.', "/");

    const SOURCE_ROOTS: &[&str] = &[
        "src/main/java",
        "src/main/kotlin",
        "src",
        "app/src/main/java",
        "app/src/main/kotlin",
    ];

    for src_root in SOURCE_ROOTS {
        let base = normalize_path(&root.join(src_root).join(&rel));
        let candidates: Vec<PathBuf> = extensions
            .iter()
            .map(|ext| base.with_extension(ext))
            .collect();
        if let Some(path) = probe(&candidates, known_files) {
            return Some(path);
        }
    }
    None
}
