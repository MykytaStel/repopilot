//! Tests for hidden-suggestion summaries and applying a visibility profile to a
//! whole `ScanSummary`. Per-finding `classify_visibility` cases live in `tests`.

use super::*;
use crate::findings::types::{Confidence, FindingCategory, Severity};
use crate::scan::types::{ScanArtifacts, ScanMetadata, ScanMetrics};

fn finding(rule_id: &str, category: FindingCategory, severity: Severity) -> Finding {
    let mut finding = Finding {
        rule_id: rule_id.to_string(),
        category,
        severity,
        confidence: Confidence::High,
        ..Default::default()
    };
    finding.populate_rule_metadata();
    finding
}

#[test]
fn hidden_suggestion_summary_groups_by_intent_rule_and_reason() {
    let findings = vec![
        finding(
            "architecture.large-file",
            FindingCategory::Architecture,
            Severity::High,
        ),
        finding(
            "architecture.large-file",
            FindingCategory::Architecture,
            Severity::High,
        ),
        finding(
            "testing.source-without-test",
            FindingCategory::Testing,
            Severity::Low,
        ),
    ];

    let summaries = build_hidden_suggestion_summaries(&findings);

    assert_eq!(summaries.len(), 2);
    assert_eq!(summaries[0].rule_id, "architecture.large-file");
    assert_eq!(summaries[0].intent, "maintainability");
    assert_eq!(summaries[0].count, 2);
    assert_eq!(summaries[1].rule_id, "testing.source-without-test");
    assert_eq!(summaries[1].intent, "testing-gap");
    assert_eq!(summaries[1].count, 1);
}

#[test]
fn apply_default_profile_recomputes_visible_counts_and_hidden_breakdown() {
    let visible = finding(
        "security.secret-candidate",
        FindingCategory::Security,
        Severity::High,
    );
    let hidden = finding(
        "architecture.large-file",
        FindingCategory::Architecture,
        Severity::High,
    );

    let mut summary = ScanSummary {
        metadata: ScanMetadata {
            ..Default::default()
        },
        metrics: ScanMetrics {
            non_empty_lines: 1_000,
            ..Default::default()
        },
        artifacts: ScanArtifacts {
            hidden_suggestions: Vec::new(),
            findings: vec![visible, hidden],
            ..Default::default()
        },
    };

    apply_visibility_profile(&mut summary, FindingVisibilityProfile::Default);

    assert_eq!(summary.visibility_profile.as_deref(), Some("default"));
    assert_eq!(summary.metrics.raw_findings_count, 2);
    assert_eq!(summary.metrics.visible_findings_count, 1);
    assert_eq!(summary.metrics.hidden_suggestions_count, 1);
    assert_eq!(summary.artifacts.raw_signal_quality.findings_total, 2);
    assert_eq!(summary.artifacts.visible_signal_quality.findings_total, 1);
    assert_eq!(summary.artifacts.signal_quality.findings_total, 1);
    assert_eq!(summary.artifacts.hidden_suggestions.len(), 1);
    assert_eq!(
        summary.artifacts.hidden_suggestions[0].intent,
        "maintainability"
    );
    assert_eq!(
        summary.artifacts.hidden_suggestions[0].rule_id,
        "architecture.large-file"
    );
    assert_eq!(summary.artifacts.findings.len(), 1);
    assert_eq!(
        summary.artifacts.findings[0].rule_id,
        "security.secret-candidate"
    );
}

#[test]
fn strict_profile_keeps_all_findings_and_clears_hidden_breakdown() {
    let mut summary = ScanSummary {
        metadata: ScanMetadata {
            ..Default::default()
        },
        metrics: ScanMetrics {
            non_empty_lines: 1_000,
            ..Default::default()
        },
        artifacts: ScanArtifacts {
            hidden_suggestions: vec![HiddenSuggestionSummary {
                intent: "maintainability".to_string(),
                rule_id: "architecture.large-file".to_string(),
                category: "architecture".to_string(),
                reason: "example".to_string(),
                count: 1,
            }],
            findings: vec![
                finding(
                    "security.secret-candidate",
                    FindingCategory::Security,
                    Severity::High,
                ),
                finding(
                    "architecture.large-file",
                    FindingCategory::Architecture,
                    Severity::High,
                ),
            ],
            ..Default::default()
        },
    };

    apply_visibility_profile(&mut summary, FindingVisibilityProfile::Strict);

    assert_eq!(summary.visibility_profile.as_deref(), Some("strict"));
    assert_eq!(summary.metrics.raw_findings_count, 2);
    assert_eq!(summary.metrics.visible_findings_count, 2);
    assert_eq!(summary.metrics.hidden_suggestions_count, 0);
    assert!(summary.artifacts.hidden_suggestions.is_empty());
    assert_eq!(summary.artifacts.findings.len(), 2);
}

#[test]
fn strict_profile_preserves_heuristic_finding_ids() {
    let expected_ids = [
        "architecture.barrel-file-risk",
        "architecture.deep-directory-nesting",
        "architecture.too-many-modules",
    ];
    let mut summary = ScanSummary {
        artifacts: ScanArtifacts {
            findings: expected_ids
                .iter()
                .map(|rule_id| finding(rule_id, FindingCategory::Architecture, Severity::Medium))
                .collect(),
            ..Default::default()
        },
        ..Default::default()
    };

    apply_visibility_profile(&mut summary, FindingVisibilityProfile::Strict);

    let actual_ids = summary
        .artifacts
        .findings
        .iter()
        .map(|finding| finding.rule_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(actual_ids, expected_ids);
}
