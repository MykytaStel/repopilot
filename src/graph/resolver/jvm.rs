//! JVM (Java / Kotlin) import resolution from fully-qualified class names.
//!
//! A JVM import names a type, not a path: `com.example.Foo` says nothing about
//! which module directory holds it. The mapping back to a file only works
//! because Maven and Gradle both put sources under `<module>/src/<set>/<lang>/`
//! and then mirror the package as directories. So the *tail* of the path is
//! fully determined by the import, and only the module prefix is unknown.
//!
//! That is why this resolver matches on the path tail instead of guessing
//! module prefixes. A fixed list of source roots resolves single-module
//! projects and nothing else: Now in Android puts its code in 68 source roots
//! like `core/data/src/main/kotlin` and `feature/foryou/impl/src/main/kotlin`,
//! and a root-relative probe misses every one of them.

use super::normalize_path;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// How many trailing segments may be dropped while looking for the declaring
/// file. `Detector.Companion.ISSUE` is three segments past `Detector.kt`, and
/// two drops reach it; more would start matching packages instead of types.
const MAX_MEMBER_SEGMENTS: usize = 2;

/// Resolves a fully-qualified JVM class name (`com.example.Foo`) to a source
/// file whose path ends with that package path.
///
/// Ambiguity is not resolved by guessing. When a class name appears under two
/// different source sets — the same type in `src/main` and `src/demo` product
/// flavors, for instance — this returns `None` so the import is recorded as
/// unresolved rather than wired to an arbitrary one of them.
pub(super) fn resolve_jvm(
    raw: &str,
    known_files: &HashSet<PathBuf>,
    extensions: &[&str],
) -> Option<PathBuf> {
    for type_path in type_path_candidates(raw) {
        // The file name is the cheap discriminator: it must equal the last
        // package segment plus a source extension. Checking it first keeps the
        // allocating full-path comparison off the overwhelming majority of
        // files, which matters because most imports in a JVM project name a
        // third-party type that no repository file declares.
        let type_name = type_path.rsplit('/').next().unwrap_or(&type_path);
        let mut matches = known_files
            .iter()
            .filter(|path| file_name_declares(path, type_name, extensions))
            .filter(|path| declares_type(path, &type_path, extensions))
            .collect::<Vec<_>>();
        if matches.is_empty() {
            continue;
        }
        // A test or fixture source set never shadows production code; only when
        // every candidate is a test set does one of those stay eligible.
        let production = matches
            .iter()
            .filter(|path| !is_test_source_set(path))
            .count();
        if production > 0 {
            matches.retain(|path| !is_test_source_set(path));
        }
        if matches.len() == 1 {
            return Some(normalize_path(matches[0]));
        }
        // Two source sets declare the same type; the build picks one by variant
        // and this resolver cannot know which.
        return None;
    }
    None
}

/// The package paths that could name the declaring file, longest first:
/// the import itself, then the same import with trailing member segments
/// dropped while the remaining tail still looks like a type name.
fn type_path_candidates(raw: &str) -> Vec<String> {
    let mut segments = raw.split('.').filter(|s| !s.is_empty()).collect::<Vec<_>>();
    if segments.len() < 2 || segments.last().is_some_and(|last| *last == "*") {
        return Vec::new();
    }
    let mut candidates = vec![segments.join("/")];
    for _ in 0..MAX_MEMBER_SEGMENTS {
        segments.pop();
        match segments.last() {
            Some(last) if starts_uppercase(last) && segments.len() >= 2 => {
                candidates.push(segments.join("/"));
            }
            _ => break,
        }
    }
    candidates
}

/// Allocation-free pre-filter: does this file's name match `<type_name>.<ext>`?
fn file_name_declares(path: &Path, type_name: &str, extensions: &[&str]) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_prefix(type_name))
        .and_then(|rest| rest.strip_prefix('.'))
        .is_some_and(|extension| extensions.contains(&extension))
}

/// Whether `path` is the file declaring `type_path`: its tail is exactly the
/// package path plus a source extension, starting at a directory boundary.
///
/// `known_files` only ever holds scanned repository files, so matching the tail
/// cannot reach outside the repository.
fn declares_type(path: &Path, type_path: &str, extensions: &[&str]) -> bool {
    let text = path.to_string_lossy().replace('\\', "/");
    let Some(without_extension) = extensions
        .iter()
        .find_map(|extension| text.strip_suffix(&format!(".{extension}")))
    else {
        return false;
    };
    without_extension
        .strip_suffix(type_path)
        .is_some_and(|prefix| prefix.is_empty() || prefix.ends_with('/'))
}

/// Maven and Gradle name non-production source sets `src/test`, `src/androidTest`,
/// and similar. Those hold their own copies of shared types.
fn is_test_source_set(path: &Path) -> bool {
    path.to_string_lossy()
        .replace('\\', "/")
        .split('/')
        .any(|component| {
            let lower = component.to_ascii_lowercase();
            lower.ends_with("test") || lower.ends_with("tests")
        })
}

fn starts_uppercase(segment: &str) -> bool {
    segment
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_uppercase())
}

#[cfg(test)]
mod tests;
