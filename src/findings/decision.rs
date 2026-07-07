use crate::findings::severity::Severity;
use crate::findings::types::{Confidence, Evidence, Finding};
use serde::{Deserialize, Serialize};

/// Canonical decision record: severity, confidence, evidence, and
/// recommendation unified into one structure shared by JSON, SARIF, MCP, and
/// AI context — instead of each output surface re-deriving its own view of
/// "what should I do about this finding."
///
/// `severity`/`confidence`/`evidence`/`recommendation` already exist as
/// top-level `Finding` fields; this necessarily duplicates them so a consumer
/// that only wants "the decision" can read one object. The existing
/// top-level fields stay for backward compatibility — this is additive, not
/// a replacement.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DecisionRecord {
    pub severity: Severity,
    pub confidence: Confidence,
    pub evidence: Vec<Evidence>,
    pub recommendation: String,
    /// Present for high-confidence findings (see
    /// [`crate::findings::verification::build_verification_plan`]); absent
    /// otherwise, since a low/medium-confidence finding is already flagged as
    /// uncertain and a step-by-step plan would overstate how actionable it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_plan: Option<VerificationPlan>,
}

/// Intentionally minimal shape: an ordered list of deterministic, evidence-backed
/// steps. `steps` is a reasonable v1 that can extend additively (new optional
/// fields) if richer plan structure is needed later.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationPlan {
    pub steps: Vec<String>,
}

pub fn build_decision_record(finding: &Finding) -> DecisionRecord {
    DecisionRecord {
        severity: finding.severity,
        confidence: finding.confidence,
        evidence: finding.evidence.clone(),
        recommendation: finding.recommendation_or_default().to_string(),
        verification_plan: crate::findings::verification::build_verification_plan(finding),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::findings::provenance::FindingProvenance;
    use crate::findings::types::FindingCategory;
    use std::path::PathBuf;

    #[test]
    fn build_decision_record_copies_severity_confidence_evidence_and_recommendation() {
        let finding = Finding {
            id: "rule.example:src/lib.rs:deadbeef".to_string(),
            rule_id: "rule.example".to_string(),
            title: "Example".to_string(),
            description: "desc".to_string(),
            recommendation: "fix it".to_string(),
            category: FindingCategory::Security,
            severity: Severity::High,
            confidence: Confidence::High,
            evidence: vec![Evidence {
                path: PathBuf::from("src/lib.rs"),
                line_start: 5,
                line_end: None,
                snippet: "let x = 1;".to_string(),
            }],
            workspace_package: None,
            docs_url: None,
            provenance: FindingProvenance::default(),
            risk: Default::default(),
        };

        let decision = build_decision_record(&finding);
        assert_eq!(decision.severity, Severity::High);
        assert_eq!(decision.confidence, Confidence::High);
        assert_eq!(decision.recommendation, "fix it");
        assert_eq!(decision.evidence, finding.evidence);
        assert!(
            decision.verification_plan.is_some(),
            "high-confidence findings should get a verification plan"
        );
    }

    #[test]
    fn build_decision_record_falls_back_to_generic_recommendation_when_empty() {
        let finding = Finding {
            id: "rule.example:src/lib.rs:deadbeef".to_string(),
            rule_id: "rule.example".to_string(),
            title: "Example".to_string(),
            description: "desc".to_string(),
            recommendation: String::new(),
            category: FindingCategory::Security,
            severity: Severity::Low,
            confidence: Confidence::Medium,
            evidence: vec![],
            workspace_package: None,
            docs_url: None,
            provenance: FindingProvenance::default(),
            risk: Default::default(),
        };

        let decision = build_decision_record(&finding);
        assert_eq!(decision.recommendation, Finding::GENERIC_RECOMMENDATION);
        assert!(decision.verification_plan.is_none());
    }
}
