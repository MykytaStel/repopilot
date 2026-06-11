//! Tracks imports the resolver could not map to scanned files while the
//! coupling graph is built.
//!
//! Resolved edges are proof: a cycle or fan-out claim built on them holds no
//! matter what else failed to resolve. *Unresolved relative* imports are the
//! opposite — they mark places where the graph is provably incomplete, which
//! weakens absence-based claims (dead modules, fan-in-derived instability).
//! Bare/package imports are real external dependencies and are not recorded.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportResolutionStats {
    /// Unresolved relative (`./`, `../`) imports keyed by the importing file.
    pub unresolved_relative_by_source: BTreeMap<PathBuf, Vec<String>>,
}

impl ImportResolutionStats {
    pub fn record(&mut self, source: &Path, raw_import: &str) {
        self.unresolved_relative_by_source
            .entry(source.to_path_buf())
            .or_default()
            .push(raw_import.to_string());
    }

    pub fn is_empty(&self) -> bool {
        self.unresolved_relative_by_source.is_empty()
    }

    pub fn total(&self) -> usize {
        self.unresolved_relative_by_source
            .values()
            .map(Vec::len)
            .sum()
    }

    /// True when any unresolved import's final path segment (extension
    /// stripped) matches `stem`. Such an import could plausibly target a file
    /// with that name, so "nothing imports it" cannot be claimed for it.
    pub fn could_target_stem(&self, stem: &str) -> bool {
        if stem.is_empty() {
            return false;
        }
        self.unresolved_relative_by_source
            .values()
            .flatten()
            .any(|import| {
                import_stem(import).is_some_and(|candidate| candidate.eq_ignore_ascii_case(stem))
            })
    }
}

fn import_stem(raw_import: &str) -> Option<&str> {
    let trimmed = raw_import.trim().trim_end_matches('/');
    let last = trimmed.rsplit(['/', '\\']).next()?;
    let stem = last.split('.').next().unwrap_or(last);
    if stem.is_empty() || stem == ".." {
        return None;
    }
    Some(stem)
}

pub(crate) fn is_relative_import(import: &str) -> bool {
    import.starts_with("./") || import.starts_with("../")
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
        assert_eq!(stats.unresolved_relative_by_source.len(), 2);
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
    fn import_stem_handles_trailing_slashes_and_parent_segments() {
        assert_eq!(import_stem("./components/"), Some("components"));
        assert_eq!(import_stem("../.."), None);
        assert_eq!(import_stem("./mod.rs"), Some("mod"));
    }

    #[test]
    fn relative_import_detection_matches_dot_prefixes_only() {
        assert!(is_relative_import("./a"));
        assert!(is_relative_import("../a"));
        assert!(!is_relative_import("react"));
        assert!(!is_relative_import("@scope/pkg"));
    }
}
