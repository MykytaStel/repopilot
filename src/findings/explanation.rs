use crate::findings::provenance::{AnalysisScope, FindingProvenance};
use crate::findings::types::Finding;
use crate::rules::{RuleLifecycle, SignalSource};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct FindingExplanation {
    pub claim: String,
    pub evidence_basis: FindingEvidenceBasis,
    pub limitations: Vec<String>,
    pub next_action: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct FindingEvidenceBasis {
    pub source: String,
    pub scope: String,
    pub lifecycle: String,
    pub location_count: usize,
}

impl FindingEvidenceBasis {
    pub fn summary(&self) -> String {
        let location_label = if self.location_count == 1 {
            "location"
        } else {
            "locations"
        };
        format!(
            "{} source; {} scope; {} rule; {} {}",
            self.source, self.scope, self.lifecycle, self.location_count, location_label
        )
    }
}

pub fn build_finding_explanation(finding: &Finding) -> FindingExplanation {
    let provenance = &finding.provenance;
    FindingExplanation {
        claim: finding.description.clone(),
        evidence_basis: FindingEvidenceBasis {
            source: provenance.signal_source.label().to_string(),
            scope: analysis_scope_label(provenance.analysis_scope).to_string(),
            lifecycle: provenance.rule_lifecycle.label().to_string(),
            location_count: finding.evidence.len(),
        },
        limitations: limitation_messages(provenance, finding.evidence.is_empty()),
        next_action: next_action(finding.evidence.is_empty()),
    }
}

fn limitation_messages(provenance: &FindingProvenance, evidence_missing: bool) -> Vec<String> {
    let mut limitations = vec![
        "Static evidence describes a structural signal; it does not by itself prove runtime behavior or user impact.".to_string(),
    ];

    if matches!(
        provenance.signal_source,
        SignalSource::TextHeuristic | SignalSource::Mixed
    ) {
        limitations.push(
            "The signal is heuristic or mixed; review the cited context before changing code."
                .to_string(),
        );
    }
    if matches!(
        provenance.rule_lifecycle,
        RuleLifecycle::Experimental | RuleLifecycle::Preview
    ) {
        limitations.push(format!(
            "This {} rule is still being calibrated; treat it as a review input, not an automatic merge decision.",
            provenance.rule_lifecycle.label()
        ));
    }
    if provenance.analysis_scope != AnalysisScope::File {
        limitations.push(format!(
            "The evidence was collected at {} scope; files or runtime state outside that scope may not be covered.",
            analysis_scope_label(provenance.analysis_scope)
        ));
    }
    if provenance.signal_source == SignalSource::ImportGraph {
        limitations.push(
            "Import resolution is bounded to supported local graph semantics; external, generated, or build-tagged behavior may be outside scope.".to_string(),
        );
    }
    if evidence_missing {
        limitations.push(
            "No concrete evidence location was retained, so the finding needs manual confirmation before action.".to_string(),
        );
    }
    limitations
}

fn next_action(evidence_missing: bool) -> String {
    if evidence_missing {
        "Find a concrete evidence location and confirm the finding manually before action."
            .to_string()
    } else {
        "Confirm the cited evidence, then apply the recommendation.".to_string()
    }
}

fn analysis_scope_label(scope: AnalysisScope) -> &'static str {
    match scope {
        AnalysisScope::File => "file",
        AnalysisScope::Repository => "repository",
        AnalysisScope::Workspace => "workspace",
        AnalysisScope::GitDiff => "git-diff",
        AnalysisScope::FrameworkProject => "framework-project",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::findings::provenance::FindingProvenance;
    use crate::findings::types::{Confidence, Evidence, FindingCategory, Severity};
    use std::path::PathBuf;

    fn finding_with_provenance(
        source: SignalSource,
        lifecycle: RuleLifecycle,
        scope: AnalysisScope,
        evidence: Vec<Evidence>,
    ) -> Finding {
        Finding {
            id: "rule.example:src/lib.rs:deadbeef".to_string(),
            rule_id: "rule.example".to_string(),
            title: "Example".to_string(),
            description: "The example contract is indicated by this signal.".to_string(),
            recommendation: "Confirm the contract and fix it.".to_string(),
            category: FindingCategory::Security,
            severity: Severity::High,
            confidence: Confidence::High,
            evidence,
            workspace_package: None,
            docs_url: None,
            provenance: FindingProvenance {
                detector: "rule.example".to_string(),
                signal_source: source,
                rule_lifecycle: lifecycle,
                analysis_scope: scope,
                knowledge_decision: None,
            },
            risk: Default::default(),
        }
    }

    #[test]
    fn explanation_describes_structural_basis_and_conservative_limit() {
        let finding = finding_with_provenance(
            SignalSource::Ast,
            RuleLifecycle::Stable,
            AnalysisScope::File,
            vec![Evidence {
                path: PathBuf::from("src/lib.rs"),
                line_start: 5,
                line_end: None,
                snippet: "let x = 1;".to_string(),
            }],
        );

        let explanation = build_finding_explanation(&finding);

        assert_eq!(
            explanation.claim,
            "The example contract is indicated by this signal."
        );
        assert_eq!(explanation.evidence_basis.source, "ast");
        assert_eq!(explanation.evidence_basis.scope, "file");
        assert_eq!(explanation.evidence_basis.lifecycle, "stable");
        assert_eq!(explanation.evidence_basis.location_count, 1);
        assert!(
            explanation
                .limitations
                .iter()
                .any(|limit| limit.contains("runtime behavior"))
        );
        assert_eq!(
            explanation.next_action,
            "Confirm the cited evidence, then apply the recommendation."
        );
    }

    #[test]
    fn explanation_marks_heuristic_preview_and_non_file_limits() {
        let finding = finding_with_provenance(
            SignalSource::TextHeuristic,
            RuleLifecycle::Preview,
            AnalysisScope::Repository,
            vec![Evidence {
                path: PathBuf::from("README.md"),
                line_start: 2,
                line_end: None,
                snippet: "possible signal".to_string(),
            }],
        );

        let explanation = build_finding_explanation(&finding);

        assert!(
            explanation
                .limitations
                .iter()
                .any(|limit| limit.contains("heuristic"))
        );
        assert!(
            explanation
                .limitations
                .iter()
                .any(|limit| limit.contains("preview"))
        );
        assert!(
            explanation
                .limitations
                .iter()
                .any(|limit| limit.contains("repository scope"))
        );
    }

    #[test]
    fn explanation_marks_missing_evidence_location_as_unresolved() {
        let finding = finding_with_provenance(
            SignalSource::ImportGraph,
            RuleLifecycle::Stable,
            AnalysisScope::File,
            Vec::new(),
        );

        let explanation = build_finding_explanation(&finding);

        assert_eq!(explanation.evidence_basis.location_count, 0);
        assert!(
            explanation
                .limitations
                .iter()
                .any(|limit| limit.contains("No concrete evidence location"))
        );
    }
}
