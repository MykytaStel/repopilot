//! Composite enrichment for boundary signals — the part that's unique to a tool
//! sitting on the diff *and* the import graph *and* file classification at once.
//!
//! - [`enrich_blast_radius`]: how far does the changed boundary file reach?
//! - [`missing_test_for_code_boundary`]: did a *code* boundary change while no
//!   test moved? (The reviewer's instinct: "you changed auth and touched no test.")

use super::BoundarySignal;
use crate::audits::context::classify::helpers::is_test_file;
use crate::review::diff::ChangedFile;
use crate::review::paths::normalized_review_path;
use crate::scan::types::CouplingGraph;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Invert the coupling graph: map each imported file to the set of files that
/// import it, with paths normalized relative to the repo root.
pub fn build_importers_by_target(
    graph: &CouplingGraph,
    repo_root: &Path,
) -> BTreeMap<PathBuf, BTreeSet<PathBuf>> {
    let mut importers: BTreeMap<PathBuf, BTreeSet<PathBuf>> = BTreeMap::new();
    for (source, targets) in &graph.edges {
        let source = normalized_review_path(source, repo_root);
        for target in targets {
            importers
                .entry(normalized_review_path(target, repo_root))
                .or_default()
                .insert(source.clone());
        }
    }
    importers
}

/// Set each signal's `blast_radius` to the number of files importing it. No-op
/// when there is no import graph (e.g. a language we don't graph yet).
pub fn enrich_blast_radius(
    signals: &mut [BoundarySignal],
    graph: Option<&CouplingGraph>,
    repo_root: &Path,
) {
    let Some(graph) = graph else {
        return;
    };
    let importers = build_importers_by_target(graph, repo_root);
    for signal in signals.iter_mut() {
        let normalized = normalized_review_path(Path::new(&signal.path), repo_root);
        signal.blast_radius = importers.get(&normalized).map_or(0, BTreeSet::len);
    }
}

/// Whether any changed file is a test file.
pub fn any_test_changed(changed_files: &[ChangedFile]) -> bool {
    changed_files
        .iter()
        .any(|file| is_test_file(&file.path, false))
}

/// True when a *code* boundary (auth / request-trust) changed but no test file
/// moved in the same diff — the "changed auth, touched no test" signal.
pub fn missing_test_for_code_boundary(
    signals: &[BoundarySignal],
    changed_files: &[ChangedFile],
) -> bool {
    signals
        .iter()
        .any(|signal| signal.category.is_code_boundary())
        && !any_test_changed(changed_files)
}
