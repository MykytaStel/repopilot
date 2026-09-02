//! Capability policy for graph-backed claims.

use super::GraphCapabilities;
use crate::graph::ImportResolutionStats;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphClaim<'a> {
    Presence,
    RepositoryAbsence,
    /// Nothing in the repository points at this file. `stem` is its file stem
    /// and `extension` its language, which decides how well the graph maps it.
    TargetAbsence {
        stem: &'a str,
        extension: Option<&'a str>,
    },
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
    /// More of this language's internal imports failed to resolve than
    /// succeeded, so the graph is not a usable map of it.
    LanguageUnmapped,
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
            Self::Unavailable {
                reason: GraphReadinessReason::LanguageUnmapped,
                ..
            } => "graph.language-unmapped",
        }
    }
}

// Absence claims are meaningful only when resolved edges cover at least the
// unresolved internal imports. No unresolved imports means the graph is mapped.
fn language_is_mapped(
    capabilities: &GraphCapabilities,
    resolution: &ImportResolutionStats,
    extension: &str,
) -> bool {
    let unresolved = resolution.total_for_extension(extension);
    unresolved == 0 || capabilities.resolved_edges_for_extension(extension) >= unresolved
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

    if let GraphClaim::TargetAbsence { stem, extension } = claim {
        if resolution.could_target_stem(stem) {
            return GraphReadiness::Unavailable {
                unresolved_internal,
                reason: GraphReadinessReason::UnresolvedTarget,
            };
        }
        if let Some(extension) = extension
            && !language_is_mapped(capabilities, resolution, extension)
        {
            return GraphReadiness::Unavailable {
                unresolved_internal,
                reason: GraphReadinessReason::LanguageUnmapped,
            };
        }
    }

    if unresolved_internal > 0 {
        GraphReadiness::Limited {
            unresolved_internal,
        }
    } else {
        GraphReadiness::Available
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::ImportResolutionStats;
    use std::path::Path;

    fn capabilities(resolved_by_extension: &[(&str, usize)]) -> GraphCapabilities {
        GraphCapabilities {
            file_nodes: 1,
            resolved_dependency_edges: resolved_by_extension.iter().map(|(_, n)| n).sum(),
            resolved_dependency_edges_by_extension: resolved_by_extension
                .iter()
                .map(|(extension, count)| ((*extension).to_string(), *count))
                .collect(),
            ..GraphCapabilities::default()
        }
    }

    fn unresolved(entries: &[(&str, usize)]) -> ImportResolutionStats {
        let mut resolution = ImportResolutionStats::default();
        for (source, count) in entries {
            for index in 0..*count {
                resolution.record(Path::new(source), &format!("./missing-{index}"));
            }
        }
        resolution
    }

    fn absence(extension: &str) -> GraphClaim<'_> {
        GraphClaim::TargetAbsence {
            stem: "unrelated",
            extension: Some(extension),
        }
    }

    #[test]
    fn a_language_with_more_holes_than_roads_cannot_support_an_absence_claim() {
        // Now in Android's shape: Kotlin resolves a minority of its imports, so
        // "nothing imports this file" carries no information about Kotlin.
        let capabilities = capabilities(&[("kt", 400)]);
        let resolution = unresolved(&[("app/Main.kt", 1666)]);

        assert_eq!(
            graph_readiness(&capabilities, &resolution, absence("kt")),
            GraphReadiness::Unavailable {
                unresolved_internal: 1666,
                reason: GraphReadinessReason::LanguageUnmapped,
            }
        );
        assert_eq!(
            GraphReadiness::Unavailable {
                unresolved_internal: 1666,
                reason: GraphReadinessReason::LanguageUnmapped,
            }
            .reason_code(),
            "graph.language-unmapped"
        );
    }

    #[test]
    fn one_unmapped_language_does_not_silence_another_in_the_same_repository() {
        // The false-negative guard: a Gradle project's TypeScript tooling can
        // resolve cleanly while its Kotlin does not.
        let capabilities = capabilities(&[("kt", 10), ("ts", 500)]);
        let resolution = unresolved(&[("app/Main.kt", 900), ("web/app.ts", 3)]);

        assert!(matches!(
            graph_readiness(&capabilities, &resolution, absence("kt")),
            GraphReadiness::Unavailable {
                reason: GraphReadinessReason::LanguageUnmapped,
                ..
            }
        ));
        assert!(matches!(
            graph_readiness(&capabilities, &resolution, absence("ts")),
            GraphReadiness::Limited { .. }
        ));
    }

    #[test]
    fn a_language_that_never_failed_to_resolve_stays_mapped_even_with_few_edges() {
        // Sparse is not unmapped: a Go package importing only the standard
        // library has no internal imports to fail.
        let capabilities = capabilities(&[("go", 0)]);
        let resolution = unresolved(&[("web/app.ts", 5)]);

        assert!(matches!(
            graph_readiness(&capabilities, &resolution, absence("go")),
            GraphReadiness::Limited { .. }
        ));
    }

    #[test]
    fn a_language_resolving_at_least_as_much_as_it_misses_stays_mapped() {
        let capabilities = capabilities(&[("py", 50)]);
        let resolution = unresolved(&[("pkg/app.py", 50)]);

        assert!(matches!(
            graph_readiness(&capabilities, &resolution, absence("py")),
            GraphReadiness::Limited { .. }
        ));
    }

    #[test]
    fn presence_claims_are_never_gated_on_resolution_quality() {
        // A resolved edge is proof on its own; only absence needs a good map.
        let capabilities = capabilities(&[("kt", 1)]);
        let resolution = unresolved(&[("app/Main.kt", 5000)]);

        assert_eq!(
            graph_readiness(&capabilities, &resolution, GraphClaim::Presence),
            GraphReadiness::Available
        );
    }
}
