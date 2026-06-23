//! Composite enrichment for boundary signals — the part that's unique to a tool
//! sitting on the diff *and* the import graph *and* file classification at once.
//!
//! - [`enrich_blast_radius`]: how far does the changed boundary file reach?
//! - [`missing_test_for_code_boundary`]: did a *code* boundary change while no
//!   test moved? (The reviewer's instinct: "you changed auth and touched no test.")

use super::BoundarySignal;
use crate::audits::context::classify::helpers::is_test_file;
use crate::graph::v2::{build_coupling_graph_snapshot, direct_dependents};
use crate::review::diff::ChangedFile;
use crate::review::paths::normalized_review_path;
use crate::scan::types::CouplingGraph;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Invert the coupling graph: map each imported file to the set of files that
/// import it, with paths normalized relative to the repo root. Sources the
/// importer relation from the shared graph v2 `GraphSnapshot` (one-hop
/// `direct_dependents`) so review shares the same graph model as the rules;
/// targets with no importers are omitted, matching the previous edge inversion.
pub fn build_importers_by_target(
    graph: &CouplingGraph,
    repo_root: &Path,
) -> BTreeMap<PathBuf, BTreeSet<PathBuf>> {
    let (snapshot, path_by_id) = build_coupling_graph_snapshot(graph);
    let mut importers: BTreeMap<PathBuf, BTreeSet<PathBuf>> = BTreeMap::new();

    for (target_id, source_ids) in direct_dependents(&snapshot) {
        if source_ids.is_empty() {
            continue;
        }
        let Some(target) = path_by_id.get(&target_id) else {
            continue;
        };
        let entry = importers
            .entry(normalized_review_path(target, repo_root))
            .or_default();
        for source_id in &source_ids {
            if let Some(source) = path_by_id.get(source_id) {
                entry.insert(normalized_review_path(source, repo_root));
            }
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
    changed_files.iter().any(|file| is_test_file(&file.path))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn coupling_graph(edges: &[(&str, &str)]) -> CouplingGraph {
        let mut edge_map: BTreeMap<PathBuf, BTreeSet<PathBuf>> = BTreeMap::new();
        let mut nodes: BTreeSet<PathBuf> = BTreeSet::new();
        for (src, dst) in edges {
            let src = PathBuf::from(src);
            let dst = PathBuf::from(dst);
            nodes.insert(src.clone());
            nodes.insert(dst.clone());
            edge_map.entry(src).or_default().insert(dst);
        }
        CouplingGraph {
            deferred_edges: Default::default(),
            edges: edge_map,
            nodes,
        }
    }

    #[test]
    fn importers_by_target_inverts_edges_with_normalized_paths() {
        // a imports b and c; b imports c. c is imported by {a, b}, b by {a},
        // and a (no importers) is absent from the map.
        let root = Path::new("/repo");
        let graph = coupling_graph(&[("a.rs", "b.rs"), ("a.rs", "c.rs"), ("b.rs", "c.rs")]);

        let importers = build_importers_by_target(&graph, root);

        let norm = |path: &str| normalized_review_path(Path::new(path), root);
        assert_eq!(
            importers.get(&norm("c.rs")),
            Some(&BTreeSet::from([norm("a.rs"), norm("b.rs")]))
        );
        assert_eq!(
            importers.get(&norm("b.rs")),
            Some(&BTreeSet::from([norm("a.rs")]))
        );
        assert!(!importers.contains_key(&norm("a.rs")));
    }
}
