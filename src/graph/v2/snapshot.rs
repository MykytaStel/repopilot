use super::{GraphDiagnostic, GraphEdge, GraphNode, GraphNodeId};
use std::collections::BTreeMap;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GraphSnapshot {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub diagnostics: Vec<GraphDiagnostic>,
    /// File node id → the workspace `Package` node it belongs to, by longest
    /// path prefix. Empty when the root is not a workspace. Carried alongside
    /// the nodes (rather than as `Contains` edges) because consumers look up
    /// membership by file, not by traversal.
    pub package_membership: BTreeMap<GraphNodeId, GraphNodeId>,
}

impl GraphSnapshot {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn diagnostic_count(&self) -> usize {
        self.diagnostics.len()
    }

    pub fn add_node(&mut self, node: GraphNode) {
        self.nodes.push(node);
    }

    pub fn add_edge(&mut self, edge: GraphEdge) {
        self.edges.push(edge);
    }

    pub fn add_diagnostic(&mut self, diagnostic: GraphDiagnostic) {
        self.diagnostics.push(diagnostic);
    }
}
