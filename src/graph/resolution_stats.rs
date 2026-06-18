//! Tracks imports the resolver could not map to scanned files while the
//! coupling graph is built.
//!
//! Resolved edges are proof: a cycle or fan-out claim built on them holds no
//! matter what else failed to resolve. *Unresolved internal* imports are the
//! opposite — they mark places where the graph is provably incomplete, which
//! weakens absence-based claims (dead modules, fan-in-derived instability).
//! Genuine third-party packages (`react`, `numpy`) are real external
//! dependencies and are not recorded; only imports that *should* have resolved
//! to a scanned file are — relative imports, recognized local path aliases, and
//! bare imports whose leading segment names a directory that exists in the
//! repository (a monorepo/workspace package the resolver did not wire up).

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportResolutionStats {
    /// Unresolved internal imports keyed by the importing file.
    pub unresolved_internal_by_source: BTreeMap<PathBuf, Vec<String>>,
}

impl ImportResolutionStats {
    pub fn record(&mut self, source: &Path, raw_import: &str) {
        self.unresolved_internal_by_source
            .entry(source.to_path_buf())
            .or_default()
            .push(raw_import.to_string());
    }

    pub fn is_empty(&self) -> bool {
        self.unresolved_internal_by_source.is_empty()
    }

    pub fn total(&self) -> usize {
        self.unresolved_internal_by_source
            .values()
            .map(Vec::len)
            .sum()
    }

    /// True when any unresolved import could plausibly target a file named
    /// `stem`, so "nothing imports it" cannot be claimed for it. Both the last
    /// path segment (`./legacy/Utils.js` → `Utils`) and the last dotted segment
    /// (Python `app.services.foo` → `foo`) are considered, since the path/module
    /// separator differs by language.
    pub fn could_target_stem(&self, stem: &str) -> bool {
        if stem.is_empty() {
            return false;
        }
        self.unresolved_internal_by_source
            .values()
            .flatten()
            .any(|import| {
                import_target_stems(import)
                    .iter()
                    .any(|candidate| candidate.eq_ignore_ascii_case(stem))
            })
    }
}

/// Candidate file stems an import could be referring to. A path import's target
/// is its last `/`-segment with the extension stripped; a dotted module import's
/// target is its last `.`-segment. Both are returned because `.` doubles as a
/// file extension and a Python/JS module separator.
fn import_target_stems(raw_import: &str) -> Vec<String> {
    let trimmed = raw_import.trim().trim_end_matches('/');
    let last_path = trimmed.rsplit(['/', '\\']).next().unwrap_or(trimmed);

    let mut stems = Vec::new();
    // Path interpretation: strip a file extension (`Button.tsx` → `Button`).
    if let Some(head) = last_path.split('.').next()
        && !head.is_empty()
        && head != ".."
    {
        stems.push(head.to_string());
    }
    // Dotted-module interpretation: the final segment (`app.services.foo`→`foo`).
    if let Some(tail) = last_path.rsplit('.').next()
        && !tail.is_empty()
        && tail != ".."
        && !stems.iter().any(|s| s == tail)
    {
        stems.push(tail.to_string());
    }
    stems
}

pub(crate) fn is_relative_import(import: &str) -> bool {
    import.starts_with("./") || import.starts_with("../")
}

/// Whether an unresolved import should weaken absence claims (dead module,
/// instability). Relative imports and recognizable local path aliases always
/// count; a bare import counts only when its leading segment names a directory
/// that exists in `repo_dirs` — i.e. an internal monorepo/workspace package the
/// resolver did not wire up — which keeps genuine third-party packages out.
pub(crate) fn is_unresolved_internal_import(import: &str, repo_dirs: &HashSet<String>) -> bool {
    let import = import.trim();
    if import.is_empty() {
        return false;
    }
    if is_relative_import(import) {
        return true;
    }
    // Common bundler/tsconfig path aliases for the project's own source root.
    if import.starts_with("@/") || import.starts_with("~/") || import == "~" {
        return true;
    }
    leading_segment(import).is_some_and(|segment| repo_dirs.contains(segment))
}

/// The first non-empty segment of an import, splitting on every path/module
/// separator used across the supported languages (`/`, `\`, `.`, `:`). For
/// `app.services.x` this is `app`; for Rust `other_crate::module` it is
/// `other_crate`; for `@angular/core` it is `@angular`.
fn leading_segment(import: &str) -> Option<&str> {
    import
        .split(['/', '\\', '.', ':'])
        .find(|segment| !segment.is_empty())
}

/// Directory names that appear anywhere in the scanned file paths. Used to tell
/// an unresolved *internal* import apart from a third-party package.
pub(crate) fn repo_directory_names<'a, I>(paths: I) -> HashSet<String>
where
    I: IntoIterator<Item = &'a Path>,
{
    let mut dirs = HashSet::new();
    for path in paths {
        let mut components: Vec<&str> = path
            .components()
            .filter_map(|component| component.as_os_str().to_str())
            .collect();
        components.pop(); // Drop the file name; keep only directory segments.
        for component in components {
            dirs.insert(component.to_string());
        }
    }
    dirs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_counts_unresolved_imports_per_source() {
        let mut stats = ImportResolutionStats::default();
        assert!(stats.is_empty());

        stats.record(Path::new("src/a.ts"), "./missing");
        stats.record(Path::new("src/a.ts"), "../gone/helper");
        stats.record(Path::new("src/b.ts"), "./missing");

        assert!(!stats.is_empty());
        assert_eq!(stats.total(), 3);
        assert_eq!(stats.unresolved_internal_by_source.len(), 2);
    }

    #[test]
    fn could_target_stem_matches_final_segment_without_extension() {
        let mut stats = ImportResolutionStats::default();
        stats.record(Path::new("src/a.ts"), "../legacy/Utils.js");

        assert!(stats.could_target_stem("utils"));
        assert!(stats.could_target_stem("Utils"));
        assert!(!stats.could_target_stem("legacy"));
        assert!(!stats.could_target_stem(""));
    }

    #[test]
    fn could_target_stem_matches_last_dotted_module_segment() {
        let mut stats = ImportResolutionStats::default();
        stats.record(Path::new("apps/web/main.py"), "app.services.foo");

        assert!(stats.could_target_stem("foo"));
        assert!(stats.could_target_stem("app"));
        assert!(!stats.could_target_stem("services"));
    }

    #[test]
    fn relative_import_detection_matches_dot_prefixes_only() {
        assert!(is_relative_import("./a"));
        assert!(is_relative_import("../a"));
        assert!(!is_relative_import("react"));
        assert!(!is_relative_import("@scope/pkg"));
    }

    #[test]
    fn internal_import_classifier_separates_workspace_from_third_party() {
        let repo_dirs: HashSet<String> = ["app", "components", "ml"]
            .into_iter()
            .map(String::from)
            .collect();

        // Relative and aliased imports are always internal.
        assert!(is_unresolved_internal_import("./helper", &repo_dirs));
        assert!(is_unresolved_internal_import(
            "@/components/Button",
            &repo_dirs
        ));
        assert!(is_unresolved_internal_import("~/lib/util", &repo_dirs));
        // Bare imports whose leading segment is a repo directory are internal.
        assert!(is_unresolved_internal_import("app.ml.train", &repo_dirs));
        assert!(is_unresolved_internal_import(
            "components/Button",
            &repo_dirs
        ));
        // Genuine third-party packages stay out.
        assert!(!is_unresolved_internal_import("react", &repo_dirs));
        assert!(!is_unresolved_internal_import("@angular/core", &repo_dirs));
        assert!(!is_unresolved_internal_import("numpy", &repo_dirs));
        assert!(!is_unresolved_internal_import(
            "django.db.models",
            &repo_dirs
        ));
    }

    #[test]
    fn repo_directory_names_collects_parent_segments_only() {
        let paths = [
            Path::new("apps/ml/app/train.py"),
            Path::new("apps/web/src/index.ts"),
        ];
        let dirs = repo_directory_names(paths);

        assert!(dirs.contains("apps"));
        assert!(dirs.contains("ml"));
        assert!(dirs.contains("app"));
        assert!(dirs.contains("web"));
        assert!(dirs.contains("src"));
        // File names are not directories.
        assert!(!dirs.contains("train"));
        assert!(!dirs.contains("index"));
    }
}
