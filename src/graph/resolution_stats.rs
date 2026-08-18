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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UnresolvedImportKind {
    RelativePath,
    LocalAlias,
    WorkspacePackage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UnresolvedImportLimitation {
    AmbiguousTarget,
    /// A Python `from <package> import name` candidate. `name` may be a
    /// submodule or a symbol defined in the package, and the two forms are
    /// indistinguishable from the import text alone.
    PythonPackageMember,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum UnresolvedImportProof {
    DefinitiveLocalCandidates(Vec<PathBuf>),
    Limited(UnresolvedImportLimitation),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct UnresolvedImportEvidence {
    pub source: PathBuf,
    pub raw_import: String,
    pub kind: UnresolvedImportKind,
    pub proof: UnresolvedImportProof,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportResolutionStats {
    /// Unresolved internal imports keyed by the importing file.
    pub unresolved_internal_by_source: BTreeMap<PathBuf, Vec<UnresolvedImportEvidence>>,
}

impl ImportResolutionStats {
    pub fn record(&mut self, source: &Path, raw_import: &str) {
        self.insert(UnresolvedImportEvidence {
            source: source.to_path_buf(),
            raw_import: raw_import.to_string(),
            kind: unresolved_import_kind(raw_import),
            proof: UnresolvedImportProof::Limited(UnresolvedImportLimitation::AmbiguousTarget),
        });
    }

    pub fn record_classified(&mut self, source: &Path, raw_import: &str, root: &Path) {
        let proof = crate::graph::resolver::definitive_local_candidates(raw_import, source, root)
            .map(UnresolvedImportProof::DefinitiveLocalCandidates)
            .unwrap_or(UnresolvedImportProof::Limited(
                UnresolvedImportLimitation::AmbiguousTarget,
            ));
        self.insert(UnresolvedImportEvidence {
            source: source.to_path_buf(),
            raw_import: raw_import.to_string(),
            kind: unresolved_import_kind(raw_import),
            proof,
        });
    }

    fn insert(&mut self, evidence: UnresolvedImportEvidence) {
        let entries = self
            .unresolved_internal_by_source
            .entry(evidence.source.clone())
            .or_default();
        entries.push(evidence);
        entries.sort();
        entries.dedup();
    }

    pub fn evidence(&self) -> impl Iterator<Item = &UnresolvedImportEvidence> {
        self.unresolved_internal_by_source.values().flatten()
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

    /// Unresolved internal imports written by files with this extension.
    ///
    /// Resolution quality is per language, not per repository: one project can
    /// map its TypeScript perfectly while its Kotlin barely resolves, and an
    /// absence claim about a Kotlin file must be judged on the Kotlin figure.
    pub fn total_for_extension(&self, extension: &str) -> usize {
        self.unresolved_internal_by_source
            .iter()
            .filter(|(source, _)| {
                source
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_some_and(|value| value == extension)
            })
            .map(|(_, evidence)| evidence.len())
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
            .any(|evidence| {
                import_target_stems(&evidence.raw_import)
                    .iter()
                    .any(|candidate| candidate.eq_ignore_ascii_case(stem))
            })
    }
}

fn unresolved_import_kind(raw_import: &str) -> UnresolvedImportKind {
    if raw_import.starts_with('.') {
        UnresolvedImportKind::RelativePath
    } else if raw_import.starts_with("@/") || raw_import.starts_with("~/") || raw_import == "~" {
        UnresolvedImportKind::LocalAlias
    } else {
        UnresolvedImportKind::WorkspacePackage
    }
}

#[cfg(test)]
mod tests;

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
    import.starts_with('.')
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
