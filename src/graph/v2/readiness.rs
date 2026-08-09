//! Capability policy for graph-backed claims.

use super::GraphCapabilities;
use crate::graph::ImportResolutionStats;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphClaim<'a> {
    Presence,
    RepositoryAbsence,
    TargetAbsence(&'a str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphReadiness {
    Available,
    Limited {
        unresolved_internal: usize,
    },
    Unavailable {
        unresolved_internal: usize,
        reason: GraphReadinessReason,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphReadinessReason {
    NoFileGraph,
    UnresolvedTarget,
}

impl GraphReadiness {
    pub fn reason_code(self) -> &'static str {
        match self {
            Self::Available => "graph.available",
            Self::Limited { .. } => "graph.unresolved-internal-imports",
            Self::Unavailable {
                reason: GraphReadinessReason::NoFileGraph,
                ..
            } => "graph.no-file-graph",
            Self::Unavailable {
                reason: GraphReadinessReason::UnresolvedTarget,
                ..
            } => "graph.unresolved-target",
        }
    }
}

pub fn graph_readiness(
    capabilities: &GraphCapabilities,
    resolution: &ImportResolutionStats,
    claim: GraphClaim<'_>,
) -> GraphReadiness {
    if matches!(claim, GraphClaim::Presence) {
        return GraphReadiness::Available;
    }

    let unresolved_internal = resolution.total();
    if capabilities.file_nodes == 0 {
        return GraphReadiness::Unavailable {
            unresolved_internal,
            reason: GraphReadinessReason::NoFileGraph,
        };
    }

    if let GraphClaim::TargetAbsence(stem) = claim
        && resolution.could_target_stem(stem)
    {
        return GraphReadiness::Unavailable {
            unresolved_internal,
            reason: GraphReadinessReason::UnresolvedTarget,
        };
    }

    if unresolved_internal > 0 {
        GraphReadiness::Limited {
            unresolved_internal,
        }
    } else {
        GraphReadiness::Available
    }
}
