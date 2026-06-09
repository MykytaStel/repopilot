use super::GraphNodeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GraphEdgeKind {
    Imports,
    ReExports,
    DependsOn,
    TestOf,
    GeneratedFrom,
    Configures,
}

/// Where an edge's evidence came from. Lets consumers weigh how an edge was
/// derived without re-deriving it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GraphEdgeProvenance {
    /// Derived from an extracted import or module declaration.
    Import,
    /// Inferred from a test-file naming convention.
    TestHeuristic,
}

/// How strongly graph v2 trusts that an edge reflects a real relationship.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GraphEdgeConfidence {
    /// The relationship was resolved to a concrete scanned file.
    High,
    /// A well-formed external dependency we cannot resolve to a scanned file.
    Medium,
    /// An import that looked local but did not resolve to any scanned file.
    Low,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphEdge {
    pub from: GraphNodeId,
    pub to: GraphNodeId,
    pub kind: GraphEdgeKind,
    pub provenance: GraphEdgeProvenance,
    pub confidence: GraphEdgeConfidence,
}
