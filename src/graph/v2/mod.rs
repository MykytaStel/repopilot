mod algorithms;
mod builder;
mod diagnostic;
mod edge;
mod node;
mod snapshot;

pub use algorithms::{
    GraphBlastRadius, GraphCycle, GraphDegreeSummary, GraphNeighborhood, GraphV2Summary,
    NodeDegree, blast_radius, compute_degrees, find_cycles, neighborhood, summarize_graph,
    top_fan_in, top_fan_out,
};
pub use builder::graph_snapshot_from_scan;
pub use diagnostic::GraphDiagnostic;
pub use edge::{GraphEdge, GraphEdgeConfidence, GraphEdgeKind, GraphEdgeProvenance};
pub use node::{GraphNode, GraphNodeId, GraphNodeKind};
pub use snapshot::GraphSnapshot;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn node_id_stores_value_and_sorts_deterministically() {
        let first = GraphNodeId::new("file:src/a.rs");
        let second = GraphNodeId::new("file:src/b.rs");
        let mut ids = [second, first.clone()];

        ids.sort();

        assert_eq!(first.as_str(), "file:src/a.rs");
        assert_eq!(ids[0], first);
    }

    #[test]
    fn new_snapshot_is_empty() {
        let snapshot = GraphSnapshot::new();

        assert_eq!(snapshot.node_count(), 0);
        assert_eq!(snapshot.edge_count(), 0);
        assert_eq!(snapshot.diagnostic_count(), 0);
    }

    #[test]
    fn snapshot_adds_nodes_edges_and_diagnostics() {
        let source_id = GraphNodeId::new("file:src/lib.rs");
        let target_id = GraphNodeId::new("external:serde");
        let mut snapshot = GraphSnapshot::new();

        snapshot.add_node(GraphNode {
            id: source_id.clone(),
            kind: GraphNodeKind::File,
            label: "src/lib.rs".to_string(),
            path: Some(PathBuf::from("src/lib.rs")),
        });
        snapshot.add_node(GraphNode {
            id: target_id.clone(),
            kind: GraphNodeKind::ExternalDependency,
            label: "serde".to_string(),
            path: None,
        });
        snapshot.add_edge(GraphEdge {
            from: source_id.clone(),
            to: target_id.clone(),
            kind: GraphEdgeKind::DependsOn,
            provenance: GraphEdgeProvenance::Import,
            confidence: GraphEdgeConfidence::High,
        });
        snapshot.add_diagnostic(GraphDiagnostic {
            code: "graph.unresolved-import".to_string(),
            message: "could not resolve one import".to_string(),
            path: Some(PathBuf::from("src/lib.rs")),
        });

        assert_eq!(snapshot.node_count(), 2);
        assert_eq!(snapshot.edge_count(), 1);
        assert_eq!(snapshot.diagnostic_count(), 1);
        assert_eq!(
            snapshot.nodes[0].path.as_deref(),
            Some(std::path::Path::new("src/lib.rs"))
        );
        assert!(snapshot.nodes[1].path.is_none());
        assert_eq!(snapshot.edges[0].from, source_id);
        assert_eq!(snapshot.edges[0].to, target_id);
        assert_eq!(snapshot.edges[0].kind, GraphEdgeKind::DependsOn);
    }
}
