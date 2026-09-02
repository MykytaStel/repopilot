//! Graph v2 capability metadata.
//!
//! [`graph_capabilities`] reports, for one `GraphSnapshot`, which dependency
//! facts the graph actually carries — counts of file vs external nodes and of
//! dependency edges by confidence tier. Architecture audits and context
//! analysis use these capabilities to decide whether graph-backed claims are
//! ready to emit. The calculation is bounded and deterministic.

use super::{GraphEdgeConfidence, GraphNodeKind, GraphSnapshot};
use std::collections::BTreeMap;

/// What dependency facts a snapshot provides. All counts are over dependency
/// edges only (`Imports`/`ReExports`/`DependsOn`); structural edges such as
/// `TestOf` are excluded.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GraphCapabilities {
    /// File nodes resolved within the repository.
    pub file_nodes: usize,
    /// External dependency nodes (packages we do not resolve to a scanned file).
    pub external_nodes: usize,
    /// Dependency edges resolved to a scanned file (high confidence).
    pub resolved_dependency_edges: usize,
    /// Dependency edges to an external package we cannot resolve locally (medium).
    pub external_dependency_edges: usize,
    /// Local imports that looked relative but resolved to nothing (low).
    pub unresolved_local_edges: usize,
    /// Resolved dependency edges keyed by the importing file's extension.
    ///
    /// The graph maps some languages better than others in the same repository:
    /// a Gradle project's TypeScript tooling can resolve cleanly while its
    /// Kotlin barely resolves at all. A claim about one file needs to know how
    /// well *its* language is mapped, not the repository average.
    pub resolved_dependency_edges_by_extension: BTreeMap<String, usize>,
    /// Bounded diagnostics recorded while building the snapshot.
    pub diagnostics: usize,
}

impl GraphCapabilities {
    /// Whether the snapshot carries any resolved file-to-file dependency edges —
    /// the minimum for cycle, degree, and blast-radius analysis to mean anything.
    pub fn supports_dependency_analysis(&self) -> bool {
        self.resolved_dependency_edges > 0
    }

    /// Resolved dependency edges written by files with this extension.
    pub fn resolved_edges_for_extension(&self, extension: &str) -> usize {
        self.resolved_dependency_edges_by_extension
            .get(extension)
            .copied()
            .unwrap_or(0)
    }
}

/// Derive [`GraphCapabilities`] from a snapshot. Deterministic and bounded by the
/// snapshot's own size.
pub fn graph_capabilities(snapshot: &GraphSnapshot) -> GraphCapabilities {
    let mut capabilities = GraphCapabilities {
        diagnostics: snapshot.diagnostics.len(),
        ..GraphCapabilities::default()
    };

    let mut extension_by_node = BTreeMap::new();
    for node in &snapshot.nodes {
        match node.kind {
            GraphNodeKind::File => capabilities.file_nodes += 1,
            GraphNodeKind::ExternalDependency => capabilities.external_nodes += 1,
            _ => {}
        }
        if let Some(extension) = node
            .path
            .as_ref()
            .and_then(|path| path.extension())
            .and_then(|extension| extension.to_str())
        {
            extension_by_node.insert(node.id.clone(), extension.to_string());
        }
    }

    for edge in &snapshot.edges {
        if !edge.kind.is_dependency() {
            continue;
        }
        match edge.confidence {
            GraphEdgeConfidence::High => {
                capabilities.resolved_dependency_edges += 1;
                if let Some(extension) = extension_by_node.get(&edge.from) {
                    *capabilities
                        .resolved_dependency_edges_by_extension
                        .entry(extension.clone())
                        .or_insert(0) += 1;
                }
            }
            GraphEdgeConfidence::Medium => capabilities.external_dependency_edges += 1,
            GraphEdgeConfidence::Low => capabilities.unresolved_local_edges += 1,
        }
    }

    capabilities
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::v2::{
        GraphEdge, GraphEdgeConfidence, GraphEdgeKind, GraphEdgeProvenance, GraphNode, GraphNodeId,
        GraphNodeKind, GraphSnapshot,
    };

    fn file(id: &str) -> GraphNode {
        GraphNode {
            id: GraphNodeId::new(format!("file:{id}")),
            kind: GraphNodeKind::File,
            label: id.to_string(),
            path: None,
        }
    }

    fn external(name: &str) -> GraphNode {
        GraphNode {
            id: GraphNodeId::new(format!("external:{name}")),
            kind: GraphNodeKind::ExternalDependency,
            label: name.to_string(),
            path: None,
        }
    }

    fn edge(
        from: &str,
        to: &str,
        kind: GraphEdgeKind,
        confidence: GraphEdgeConfidence,
    ) -> GraphEdge {
        GraphEdge {
            from: GraphNodeId::new(from),
            to: GraphNodeId::new(to),
            kind,
            provenance: GraphEdgeProvenance::Import,
            confidence,
        }
    }

    #[test]
    fn empty_snapshot_supports_nothing() {
        let capabilities = graph_capabilities(&GraphSnapshot::new());

        assert_eq!(capabilities, GraphCapabilities::default());
        assert!(!capabilities.supports_dependency_analysis());
    }

    #[test]
    fn counts_nodes_and_dependency_edges_by_confidence() {
        let mut snapshot = GraphSnapshot::new();
        snapshot.add_node(file("a.rs"));
        snapshot.add_node(file("b.rs"));
        snapshot.add_node(external("serde"));
        // Resolved local import, external package dependency, and a structural
        // test edge that must be ignored.
        snapshot.add_edge(edge(
            "file:a.rs",
            "file:b.rs",
            GraphEdgeKind::Imports,
            GraphEdgeConfidence::High,
        ));
        snapshot.add_edge(edge(
            "file:a.rs",
            "external:serde",
            GraphEdgeKind::DependsOn,
            GraphEdgeConfidence::Medium,
        ));
        snapshot.add_edge(edge(
            "file:b.rs",
            "file:a.rs",
            GraphEdgeKind::TestOf,
            GraphEdgeConfidence::High,
        ));

        let capabilities = graph_capabilities(&snapshot);

        assert_eq!(capabilities.file_nodes, 2);
        assert_eq!(capabilities.external_nodes, 1);
        assert_eq!(capabilities.resolved_dependency_edges, 1);
        assert_eq!(capabilities.external_dependency_edges, 1);
        assert_eq!(capabilities.unresolved_local_edges, 0);
        assert!(capabilities.supports_dependency_analysis());
    }
}
