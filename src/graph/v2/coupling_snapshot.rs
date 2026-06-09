use super::{
    GraphEdge, GraphEdgeConfidence, GraphEdgeKind, GraphEdgeProvenance, GraphNode, GraphNodeId,
    GraphNodeKind, GraphSnapshot,
};
use crate::scan::types::CouplingGraph;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Re-encodes a file-level [`CouplingGraph`] as a graph v2 [`GraphSnapshot`] so
/// shared v2 algorithms (e.g. SCC cycle detection) can run over it, returning
/// the snapshot alongside a map from node id back to file path. Only file nodes
/// and dependency (`Imports`) edges are produced.
///
/// This is the shared bridge between the v1 coupling graph and graph v2. It is
/// deliberately free of audit concepts (no severities, rule ids, findings, or
/// evidence), so any future graph v2 consumer can reuse it without depending on
/// a particular rule.
pub fn build_coupling_graph_snapshot(
    graph: &CouplingGraph,
) -> (GraphSnapshot, BTreeMap<GraphNodeId, PathBuf>) {
    let node_id = |path: &Path| GraphNodeId::new(format!("file:{}", path.display()));
    let mut snapshot = GraphSnapshot::new();
    let mut path_by_id = BTreeMap::new();

    for path in &graph.nodes {
        let id = node_id(path);
        path_by_id.insert(id.clone(), path.clone());
        snapshot.add_node(GraphNode {
            id,
            kind: GraphNodeKind::File,
            label: path.display().to_string(),
            path: Some(path.clone()),
        });
    }

    for (from, targets) in &graph.edges {
        for to in targets {
            snapshot.add_edge(GraphEdge {
                from: node_id(from),
                to: node_id(to),
                kind: GraphEdgeKind::Imports,
                provenance: GraphEdgeProvenance::Import,
                confidence: GraphEdgeConfidence::High,
            });
        }
    }

    (snapshot, path_by_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::v2::find_cycles;
    use std::collections::BTreeSet;

    fn node_id(path: &str) -> GraphNodeId {
        GraphNodeId::new(format!("file:{path}"))
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
            edges: edge_map,
            nodes,
        }
    }

    #[test]
    fn empty_coupling_graph_produces_empty_snapshot() {
        let graph = CouplingGraph {
            edges: BTreeMap::new(),
            nodes: BTreeSet::new(),
        };

        let (snapshot, path_by_id) = build_coupling_graph_snapshot(&graph);

        assert_eq!(snapshot.node_count(), 0);
        assert_eq!(snapshot.edge_count(), 0);
        assert_eq!(snapshot.diagnostic_count(), 0);
        assert!(path_by_id.is_empty());
    }

    #[test]
    fn single_directional_edge_is_preserved() {
        let graph = coupling_graph(&[("src/a.rs", "src/b.rs")]);

        let (snapshot, path_by_id) = build_coupling_graph_snapshot(&graph);

        assert_eq!(snapshot.node_count(), 2);
        assert_eq!(snapshot.edge_count(), 1);
        let edge = &snapshot.edges[0];
        assert_eq!(edge.from, node_id("src/a.rs"));
        assert_eq!(edge.to, node_id("src/b.rs"));
        assert_eq!(edge.kind, GraphEdgeKind::Imports);
        // Direction is not symmetric: no reverse edge is synthesised.
        assert!(
            !snapshot
                .edges
                .iter()
                .any(|edge| edge.from == node_id("src/b.rs"))
        );
        assert_eq!(
            path_by_id.get(&node_id("src/a.rs")),
            Some(&PathBuf::from("src/a.rs"))
        );
    }

    #[test]
    fn simple_cycle_is_representable() {
        // a.rs -> b.rs and b.rs -> a.rs form one strongly-connected component.
        let graph = coupling_graph(&[("a.rs", "b.rs"), ("b.rs", "a.rs")]);

        let (snapshot, _) = build_coupling_graph_snapshot(&graph);
        let cycles = find_cycles(&snapshot);

        assert_eq!(snapshot.edge_count(), 2);
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].node_ids.len(), 2);
    }

    #[test]
    fn duplicate_edges_collapse_to_stable_output() {
        // The same edge supplied twice still yields exactly one edge, and the
        // conversion is deterministic across repeated builds.
        let graph = coupling_graph(&[("a.rs", "b.rs"), ("a.rs", "b.rs")]);

        let (first, _) = build_coupling_graph_snapshot(&graph);
        let (second, _) = build_coupling_graph_snapshot(&graph);

        assert_eq!(first.edge_count(), 1);
        assert_eq!(first, second);
    }

    #[test]
    fn node_ids_and_labels_use_repository_relative_paths() {
        let graph = coupling_graph(&[("src/a.rs", "src/nested/b.rs")]);

        let (snapshot, _) = build_coupling_graph_snapshot(&graph);

        let a = snapshot
            .nodes
            .iter()
            .find(|node| node.path.as_deref() == Some(Path::new("src/a.rs")))
            .expect("a.rs node should exist");
        assert_eq!(a.id, node_id("src/a.rs"));
        assert_eq!(a.label, "src/a.rs");
        assert_eq!(a.kind, GraphNodeKind::File);
    }
}
