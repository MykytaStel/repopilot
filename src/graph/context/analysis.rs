use super::RepoContextGraph;
use crate::graph::FileMetrics;
use crate::graph::v2::{
    GraphCapabilities, GraphDegreeSummary, GraphNodeId, GraphSnapshot, bounded_cycle_paths,
    build_coupling_graph_snapshot, compute_degrees, direct_dependents, graph_capabilities,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

pub(super) struct ContextGraphAnalysis {
    snapshot: GraphSnapshot,
    path_by_id: BTreeMap<GraphNodeId, PathBuf>,
    excluded_cycle_edges: BTreeSet<(GraphNodeId, GraphNodeId)>,
    degrees: GraphDegreeSummary,
    dependents: BTreeMap<GraphNodeId, BTreeSet<GraphNodeId>>,
    capabilities: GraphCapabilities,
}

impl ContextGraphAnalysis {
    pub(super) fn from_graph(graph: &RepoContextGraph) -> Self {
        let coupling = graph.coupling_graph();
        let excluded_cycle_edges = excluded_cycle_edges(&coupling);
        let (snapshot, path_by_id) = build_coupling_graph_snapshot(&coupling);
        let degrees = compute_degrees(&snapshot);
        let dependents = direct_dependents(&snapshot);
        let capabilities = graph_capabilities(&snapshot);
        Self {
            snapshot,
            path_by_id,
            excluded_cycle_edges,
            degrees,
            dependents,
            capabilities,
        }
    }

    pub(super) fn file_metrics(&self) -> Vec<FileMetrics> {
        self.degrees
            .nodes
            .iter()
            .filter_map(|degree| {
                let path = self.path_by_id.get(&degree.node_id)?.clone();
                Some((
                    path.clone(),
                    FileMetrics {
                        path,
                        fan_in: degree.fan_in,
                        fan_out: degree.fan_out,
                        instability: degree.instability(),
                    },
                ))
            })
            .collect::<BTreeMap<_, _>>()
            .into_values()
            .collect()
    }

    pub(super) fn direct_dependents_by_path(&self) -> BTreeMap<PathBuf, BTreeSet<PathBuf>> {
        self.dependents
            .iter()
            .filter_map(|(target_id, source_ids)| {
                let target = self.path_by_id.get(target_id)?.clone();
                let sources = source_ids
                    .iter()
                    .filter_map(|id| self.path_by_id.get(id).cloned())
                    .collect();
                Some((target, sources))
            })
            .collect()
    }

    pub(super) fn capabilities(&self) -> &GraphCapabilities {
        &self.capabilities
    }

    pub(super) fn bounded_cycles(&self, max_cycles: usize) -> BoundedContextCycles {
        let cycles = bounded_cycle_paths(&self.snapshot, &self.excluded_cycle_edges, max_cycles);
        BoundedContextCycles {
            paths: cycles
                .paths
                .into_iter()
                .map(|path| {
                    path.into_iter()
                        .filter_map(|id| self.path_by_id.get(&id).cloned())
                        .collect()
                })
                .collect(),
            truncated: cycles.truncated,
            depth_exceeded: cycles.depth_exceeded,
        }
    }
}

pub(super) struct BoundedContextCycles {
    pub(super) paths: Vec<Vec<PathBuf>>,
    pub(super) truncated: bool,
    pub(super) depth_exceeded: bool,
}

fn excluded_cycle_edges(
    graph: &crate::graph::CouplingGraph,
) -> BTreeSet<(GraphNodeId, GraphNodeId)> {
    let filtered = crate::graph::without_rust_module_containment_edges(graph);
    let node_id = |path: &std::path::Path| GraphNodeId::new(format!("file:{}", path.display()));
    let mut excluded = BTreeSet::new();
    for (source, targets) in &graph.edges {
        let deferred = graph.deferred_edges.get(source);
        let kept = filtered.edges.get(source);
        for target in targets {
            if deferred.is_some_and(|edges| edges.contains(target))
                || !kept.is_some_and(|edges| edges.contains(target))
            {
                excluded.insert((node_id(source), node_id(target)));
            }
        }
    }
    excluded
}

#[cfg(test)]
mod tests {
    use super::super::{RepoContextGraph, RepoContextNode};
    use super::ContextGraphAnalysis;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::{Path, PathBuf};

    #[test]
    fn projects_degrees_dependents_and_capabilities_by_path() {
        let graph = graph_with_edge("a.rs", "b.rs");

        let analysis = ContextGraphAnalysis::from_graph(&graph);
        let metrics = analysis.file_metrics();
        let a = metric(&metrics, "a.rs");
        let b = metric(&metrics, "b.rs");

        assert_eq!(a.fan_out, 1);
        assert_eq!(b.fan_in, 1);
        assert_eq!(
            analysis.direct_dependents_by_path()[Path::new("b.rs")],
            BTreeSet::from([PathBuf::from("a.rs")])
        );
        assert_eq!(analysis.capabilities().resolved_dependency_edges, 1);
    }

    #[test]
    fn retains_isolated_nodes_with_zero_degrees() {
        let mut graph = graph_with_edge("a.rs", "b.rs");
        graph.nodes.push(node("isolated.rs"));

        let analysis = ContextGraphAnalysis::from_graph(&graph);
        let metrics = analysis.file_metrics();
        let isolated = metric(&metrics, "isolated.rs");

        assert_eq!(isolated.fan_in, 0);
        assert_eq!(isolated.fan_out, 0);
        assert_eq!(isolated.instability, 0.0);
    }

    #[test]
    fn projections_are_deterministic_across_builds() {
        let graph = graph_with_edge("a.rs", "b.rs");

        let first = ContextGraphAnalysis::from_graph(&graph);
        let second = ContextGraphAnalysis::from_graph(&graph);

        assert_eq!(metric_keys(&first), metric_keys(&second));
        assert_eq!(
            first.direct_dependents_by_path(),
            second.direct_dependents_by_path()
        );
        assert_eq!(first.capabilities(), second.capabilities());
    }

    #[test]
    fn bounded_cycles_exclude_deferred_relationships() {
        let mut graph = graph_with_edge("a.py", "b.py");
        graph
            .edges
            .entry(PathBuf::from("b.py"))
            .or_default()
            .insert(PathBuf::from("a.py"));
        graph.deferred_edges.insert(
            PathBuf::from("b.py"),
            BTreeSet::from([PathBuf::from("a.py")]),
        );

        let cycles = ContextGraphAnalysis::from_graph(&graph).bounded_cycles(20);

        assert!(cycles.paths.is_empty());
        assert!(!cycles.truncated);
        assert!(!cycles.depth_exceeded);
    }

    fn metric_keys(analysis: &ContextGraphAnalysis) -> Vec<(PathBuf, usize, usize, u32)> {
        analysis
            .file_metrics()
            .into_iter()
            .map(|metric| {
                (
                    metric.path,
                    metric.fan_in,
                    metric.fan_out,
                    metric.instability.to_bits(),
                )
            })
            .collect()
    }

    fn metric<'a>(
        metrics: &'a [crate::graph::FileMetrics],
        path: &str,
    ) -> &'a crate::graph::FileMetrics {
        metrics
            .iter()
            .find(|metric| metric.path == Path::new(path))
            .expect("metric should exist")
    }

    fn graph_with_edge(source: &str, target: &str) -> RepoContextGraph {
        RepoContextGraph {
            root_path: PathBuf::from("."),
            nodes: vec![node(source), node(target)],
            edges: BTreeMap::from([(
                PathBuf::from(source),
                BTreeSet::from([PathBuf::from(target)]),
            )]),
            deferred_edges: BTreeMap::new(),
            detected_frameworks: Vec::new(),
            framework_projects: Vec::new(),
            react_native: None,
        }
    }

    fn node(path: &str) -> RepoContextNode {
        RepoContextNode {
            path: PathBuf::from(path),
            language: Some("Rust".to_string()),
            roles: Vec::new(),
            frameworks: Vec::new(),
            runtimes: Vec::new(),
            paradigms: Vec::new(),
            workspace_package: None,
            non_empty_lines: 1,
            imports: Vec::new(),
            deferred_imports: Vec::new(),
            is_test: false,
            is_generated: false,
            is_config: false,
        }
    }
}
