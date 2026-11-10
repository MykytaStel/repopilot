//! Bounded-depth dependency impact paths: for each changed file, which files
//! import it directly (hop 1) and transitively (hops 2..=depth), plus a
//! rolled-up affected-surface summary. Additive to [`compute_blast_radius`]
//! (which stays one-hop and untouched, since the risk overlay and existing
//! tests key off its exact semantics) — this reuses the same importer
//! relation (`composites::build_importers_by_target`) so the two are always
//! consistent, just extended past one hop.
//!
//! [`compute_blast_radius`]: super::blast_radius::compute_blast_radius

use crate::review::diff::ChangedFile;
use crate::review::paths::normalized_review_path;
use crate::review::signals::composites;
use crate::scan::types::ScanSummary;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ImpactPaths {
    pub depth: usize,
    pub files: Vec<FileImpact>,
    pub affected_surface: AffectedSurface,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct FileImpact {
    pub path: PathBuf,
    pub direct_dependents: Vec<PathBuf>,
    pub transitive_dependents: Vec<PathBuf>,
    pub routes: Vec<ImpactRoute>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ImpactRoute {
    pub dependent: PathBuf,
    /// One deterministic shortest import path, starting at the changed file.
    pub files: Vec<PathBuf>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct AffectedSurface {
    pub impacted_files: usize,
    pub affected_directories: Vec<PathBuf>,
}

/// Traces dependents of each changed file up to `depth` hops. Requires a
/// coupling graph; returns an empty result (matching
/// [`compute_blast_radius`](super::blast_radius::compute_blast_radius)) when
/// there is none, or when `depth == 0`.
pub fn compute_impact_paths(
    summary: &ScanSummary,
    repo_root: &Path,
    changed_files: &[ChangedFile],
    depth: usize,
) -> ImpactPaths {
    let empty = ImpactPaths {
        depth,
        ..Default::default()
    };
    if depth == 0 {
        return empty;
    }
    let Some(graph) = &summary.artifacts.coupling_graph else {
        return empty;
    };

    let changed_paths: BTreeSet<PathBuf> = changed_files
        .iter()
        .map(|file| normalized_review_path(&file.path, repo_root))
        .collect();
    let importers_by_target = composites::build_importers_by_target(graph, repo_root);
    let (files, all_impacted) = affected_files(&changed_paths, &importers_by_target, depth);
    ImpactPaths {
        depth,
        files,
        affected_surface: affected_surface(all_impacted),
    }
}

fn affected_files(
    changed_paths: &BTreeSet<PathBuf>,
    importers_by_target: &BTreeMap<PathBuf, BTreeSet<PathBuf>>,
    depth: usize,
) -> (Vec<FileImpact>, BTreeSet<PathBuf>) {
    let mut all_impacted = BTreeSet::new();
    let mut files = Vec::new();
    for changed in changed_paths {
        let paths = bounded_paths(changed, importers_by_target, changed_paths, depth);
        if !paths.is_empty() {
            all_impacted.extend(paths.keys().cloned());
            files.push(file_impact(changed, paths));
        }
    }
    (files, all_impacted)
}

fn file_impact(changed: &Path, paths: BTreeMap<PathBuf, Vec<PathBuf>>) -> FileImpact {
    let direct_dependents = dependents_at_depth(&paths, 2);
    let transitive_dependents = paths
        .iter()
        .filter(|(_, route)| route.len() > 2)
        .map(|(path, _)| path.clone())
        .collect();
    let routes = paths
        .into_iter()
        .map(|(dependent, files)| ImpactRoute { dependent, files })
        .collect();
    FileImpact {
        path: changed.to_path_buf(),
        direct_dependents,
        transitive_dependents,
        routes,
    }
}

fn dependents_at_depth(paths: &BTreeMap<PathBuf, Vec<PathBuf>>, depth: usize) -> Vec<PathBuf> {
    paths
        .iter()
        .filter(|(_, route)| route.len() == depth)
        .map(|(path, _)| path.clone())
        .collect()
}

fn affected_surface(all_impacted: BTreeSet<PathBuf>) -> AffectedSurface {
    let affected_directories = all_impacted
        .iter()
        .filter_map(|path| path.parent().map(PathBuf::from))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    AffectedSurface {
        impacted_files: all_impacted.len(),
        affected_directories,
    }
}

// BFS over importers up to `depth`, excluding the seed and changed files. The
// ordered maps keep the result deterministic.
fn bounded_paths(
    seed: &Path,
    importers_by_target: &BTreeMap<PathBuf, BTreeSet<PathBuf>>,
    changed_paths: &BTreeSet<PathBuf>,
    depth: usize,
) -> BTreeMap<PathBuf, Vec<PathBuf>> {
    let mut paths: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
    let mut queue: VecDeque<Vec<PathBuf>> = VecDeque::new();
    queue.push_back(vec![seed.to_path_buf()]);
    let mut visited: BTreeSet<PathBuf> = BTreeSet::from([seed.to_path_buf()]);

    while let Some(route) = queue.pop_front() {
        let Some(current) = route.last() else {
            continue;
        };
        if route.len() > depth {
            continue;
        }
        let Some(importers) = importers_by_target.get(current) else {
            continue;
        };
        for importer in importers {
            if !visited.insert(importer.clone()) {
                continue;
            }
            let mut next_route = route.clone();
            next_route.push(importer.clone());
            if !changed_paths.contains(importer) {
                paths.insert(importer.clone(), next_route.clone());
            }
            queue.push_back(next_route);
        }
    }

    paths
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::diff::ChangeStatus;
    use crate::scan::types::{CouplingGraph, ScanArtifacts, ScanMetadata, ScanSummary};
    use std::path::PathBuf;

    fn changed_file(path: &str) -> ChangedFile {
        ChangedFile {
            path: PathBuf::from(path),
            status: ChangeStatus::Modified,
            hunks: Vec::new(),
            ranges: Vec::new(),
        }
    }

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

    fn summary_with_graph(root: &Path, graph: CouplingGraph) -> ScanSummary {
        ScanSummary {
            metadata: ScanMetadata {
                root_path: root.to_path_buf(),
                ..ScanMetadata::default()
            },
            artifacts: ScanArtifacts {
                coupling_graph: Some(graph),
                ..ScanArtifacts::default()
            },
            ..ScanSummary::default()
        }
    }

    #[test]
    fn depth_two_splits_direct_and_transitive_dependents() {
        let temp = tempfile::tempdir().expect("tempdir");
        // a.rs -> b.rs -> c.rs (b and c import a and b respectively... actually
        // edges point importer -> imported per CouplingGraph's own convention
        // used by build_importers_by_target: (from imports to)).
        let graph = coupling_graph(&[("b.rs", "a.rs"), ("c.rs", "b.rs")]);
        let summary = summary_with_graph(temp.path(), graph);
        let changed = vec![changed_file("a.rs")];

        let impact = compute_impact_paths(&summary, temp.path(), &changed, 2);

        assert_eq!(impact.files.len(), 1);
        let file_impact = &impact.files[0];
        assert_eq!(file_impact.path, PathBuf::from("a.rs"));
        assert_eq!(file_impact.direct_dependents, vec![PathBuf::from("b.rs")]);
        assert_eq!(
            file_impact.transitive_dependents,
            vec![PathBuf::from("c.rs")]
        );
        assert_eq!(
            file_impact.routes,
            vec![
                ImpactRoute {
                    dependent: PathBuf::from("b.rs"),
                    files: vec![PathBuf::from("a.rs"), PathBuf::from("b.rs")],
                },
                ImpactRoute {
                    dependent: PathBuf::from("c.rs"),
                    files: vec![
                        PathBuf::from("a.rs"),
                        PathBuf::from("b.rs"),
                        PathBuf::from("c.rs"),
                    ],
                },
            ]
        );
        assert_eq!(impact.affected_surface.impacted_files, 2);
    }

    #[test]
    fn equal_length_impact_routes_choose_a_deterministic_lexical_path() {
        let temp = tempfile::tempdir().expect("tempdir");
        let graph = coupling_graph(&[
            ("c.rs", "a.rs"),
            ("b.rs", "a.rs"),
            ("d.rs", "c.rs"),
            ("d.rs", "b.rs"),
        ]);
        let summary = summary_with_graph(temp.path(), graph);
        let changed = vec![changed_file("a.rs")];

        let impact = compute_impact_paths(&summary, temp.path(), &changed, 2);

        let route = impact.files[0]
            .routes
            .iter()
            .find(|route| route.dependent.as_path() == Path::new("d.rs"))
            .expect("d.rs is reachable through both b.rs and c.rs");
        assert_eq!(
            route.files,
            vec![
                PathBuf::from("a.rs"),
                PathBuf::from("b.rs"),
                PathBuf::from("d.rs")
            ]
        );
    }

    #[test]
    fn depth_one_excludes_transitive_dependents() {
        let temp = tempfile::tempdir().expect("tempdir");
        let graph = coupling_graph(&[("b.rs", "a.rs"), ("c.rs", "b.rs")]);
        let summary = summary_with_graph(temp.path(), graph);
        let changed = vec![changed_file("a.rs")];

        let impact = compute_impact_paths(&summary, temp.path(), &changed, 1);

        let file_impact = &impact.files[0];
        assert_eq!(file_impact.direct_dependents, vec![PathBuf::from("b.rs")]);
        assert!(file_impact.transitive_dependents.is_empty());
        assert_eq!(impact.affected_surface.impacted_files, 1);
    }

    #[test]
    fn depth_zero_yields_empty_result() {
        let temp = tempfile::tempdir().expect("tempdir");
        let graph = coupling_graph(&[("b.rs", "a.rs")]);
        let summary = summary_with_graph(temp.path(), graph);
        let changed = vec![changed_file("a.rs")];

        let impact = compute_impact_paths(&summary, temp.path(), &changed, 0);

        assert!(impact.files.is_empty());
        assert_eq!(impact.affected_surface.impacted_files, 0);
    }

    #[test]
    fn no_coupling_graph_yields_empty_result() {
        let temp = tempfile::tempdir().expect("tempdir");
        let summary = ScanSummary {
            metadata: ScanMetadata {
                root_path: temp.path().to_path_buf(),
                ..ScanMetadata::default()
            },
            artifacts: ScanArtifacts {
                coupling_graph: None,
                ..ScanArtifacts::default()
            },
            ..ScanSummary::default()
        };
        let changed = vec![changed_file("a.rs")];

        let impact = compute_impact_paths(&summary, temp.path(), &changed, 3);

        assert!(impact.files.is_empty());
    }

    #[test]
    fn changed_importers_are_excluded_from_dependents() {
        let temp = tempfile::tempdir().expect("tempdir");
        // b.rs imports a.rs, and b.rs is itself changed alongside a.rs, so it
        // must not show up as a dependent of a.rs (mirrors compute_blast_radius).
        let graph = coupling_graph(&[("b.rs", "a.rs")]);
        let summary = summary_with_graph(temp.path(), graph);
        let changed = vec![changed_file("a.rs"), changed_file("b.rs")];

        let impact = compute_impact_paths(&summary, temp.path(), &changed, 3);

        assert!(impact.files.is_empty());
        assert_eq!(impact.affected_surface.impacted_files, 0);
    }

    #[test]
    fn affected_directories_are_sorted_and_deduped() {
        let temp = tempfile::tempdir().expect("tempdir");
        let graph = coupling_graph(&[
            ("src/x/b.rs", "src/a.rs"),
            ("src/y/c.rs", "src/a.rs"),
            ("src/x/d.rs", "src/a.rs"),
        ]);
        let summary = summary_with_graph(temp.path(), graph);
        let changed = vec![changed_file("src/a.rs")];

        let impact = compute_impact_paths(&summary, temp.path(), &changed, 1);

        assert_eq!(
            impact.affected_surface.affected_directories,
            vec![PathBuf::from("src/x"), PathBuf::from("src/y")]
        );
    }
}
