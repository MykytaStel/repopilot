//! Graph v2-backed per-file coupling metrics.
//!
//! [`coupling_file_metrics`] is the drop-in successor to the v1 [`compute_metrics`]
//! path: it derives fan-in, fan-out, and instability from graph v2 degree
//! counting over the shared `GraphSnapshot`, returning identical values in the
//! same repository-path order. Consumers (the import-coupling rules and the AI
//! context hot-files fallback) use this so they share one graph model.
//!
//! [`compute_metrics`]: super::compute_metrics

use super::{CouplingGraph, FileMetrics};
use crate::graph::v2::{build_coupling_graph_snapshot, compute_degrees};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Per-file fan-in, fan-out, and instability for a coupling graph, computed via
/// graph v2 degrees. One entry per file, ordered by repository-relative path.
pub fn coupling_file_metrics(graph: &CouplingGraph) -> Vec<FileMetrics> {
    let (snapshot, path_by_id) = build_coupling_graph_snapshot(graph);
    let mut metrics: BTreeMap<PathBuf, FileMetrics> = BTreeMap::new();

    for degree in compute_degrees(&snapshot).nodes {
        let Some(path) = path_by_id.get(&degree.node_id) else {
            continue;
        };
        metrics.insert(
            path.clone(),
            FileMetrics {
                path: path.clone(),
                fan_in: degree.fan_in,
                fan_out: degree.fan_out,
                instability: degree.instability(),
            },
        );
    }

    metrics.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::compute_metrics;
    use std::collections::BTreeSet;

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
            edges: edge_map,
            nodes,
        }
    }

    /// The v2-backed metrics must reproduce the v1 contract exactly: same files,
    /// same order, same fan-in/fan-out/instability bits.
    #[test]
    fn matches_v1_compute_metrics() {
        let graph = coupling_graph(&[
            ("a.rs", "b.rs"),
            ("a.rs", "c.rs"),
            ("b.rs", "c.rs"),
            ("c.rs", "a.rs"),
            ("src/lib.rs", "src/util.rs"),
        ]);

        let v2 = coupling_file_metrics(&graph);
        let v1 = compute_metrics(&graph);

        let key = |metrics: &[FileMetrics]| {
            metrics
                .iter()
                .map(|metric| {
                    (
                        metric.path.clone(),
                        metric.fan_in,
                        metric.fan_out,
                        metric.instability.to_bits(),
                    )
                })
                .collect::<Vec<_>>()
        };

        assert_eq!(key(&v2), key(&v1));
    }

    #[test]
    fn covers_isolated_targets_with_zero_instability() {
        let metrics = coupling_file_metrics(&coupling_graph(&[("a.rs", "b.rs")]));
        let b = metrics
            .iter()
            .find(|metric| metric.path == std::path::Path::new("b.rs"))
            .expect("imported file is present");

        assert_eq!(b.fan_in, 1);
        assert_eq!(b.fan_out, 0);
        assert_eq!(b.instability, 0.0);
    }
}
