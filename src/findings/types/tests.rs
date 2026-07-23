use super::*;
use crate::findings::provenance::{KnowledgeDecisionAction, KnowledgeDecisionProvenance};

// `Severity::Info` is `Severity::default()`, so `populate_rule_metadata` uses
// it as a sentinel meaning "the audit never set severity, fill in the
// registry default." These tests pin the two cases that sentinel has to
// distinguish, per the `Confidence::Medium`-as-sentinel caveat documented in
// `graph_queries/mod.rs`.

#[test]
fn populate_rule_metadata_fills_registry_default_when_severity_was_never_set() {
    let mut finding = Finding {
        rule_id: "architecture.large-file".to_string(),
        severity: Severity::Info,
        ..Finding::default()
    };
    assert!(finding.provenance.knowledge_decision.is_none());

    finding.populate_rule_metadata();

    // architecture.large-file's registry default_severity is Medium.
    assert_eq!(finding.severity, Severity::Medium);
}

#[test]
fn populate_rule_metadata_preserves_an_explicit_info_decision() {
    let mut finding = Finding {
        rule_id: "architecture.large-file".to_string(),
        severity: Severity::Info,
        ..Finding::default()
    };
    finding.provenance.knowledge_decision = Some(KnowledgeDecisionProvenance {
        base_severity: Severity::High,
        signal: None,
        action: KnowledgeDecisionAction::Downgrade,
        decided_severity: Severity::Info,
        reason: Some("legacy freeze, informational only".to_string()),
        overlay_applied: true,
    });

    finding.populate_rule_metadata();

    assert_eq!(finding.severity, Severity::Info);
}
