use super::*;
use crate::findings::types::{Confidence, Evidence, FindingCategory, Severity};
use crate::risk::{RiskAssessment, RiskPriority};
use std::path::PathBuf;

fn finding(rule_id: &str, category: FindingCategory, severity: Severity) -> Finding {
    Finding {
        rule_id: rule_id.to_string(),
        category,
        severity,
        confidence: Confidence::High,
        ..Default::default()
    }
}

fn finding_with_path(
    rule_id: &str,
    category: FindingCategory,
    severity: Severity,
    path: &str,
) -> Finding {
    let mut finding = finding(rule_id, category, severity);
    finding.evidence = vec![Evidence {
        path: PathBuf::from(path),
        line_start: 1,
        line_end: None,
        snippet: "process.exit(1);".to_string(),
    }];
    finding
}

fn finding_with_priority(
    rule_id: &str,
    category: FindingCategory,
    severity: Severity,
    priority: RiskPriority,
) -> Finding {
    let mut finding = finding(rule_id, category, severity);
    finding.risk = RiskAssessment {
        priority,
        ..RiskAssessment::default()
    };
    finding
}

#[test]
fn default_profile_hides_testing_gaps_by_intent() {
    let finding = finding(
        "testing.source-without-test",
        FindingCategory::Testing,
        Severity::Low,
    );

    let decision = classify_visibility(&finding);

    assert_eq!(decision.intent, FindingIntent::TestingGap);
    assert!(!decision.visible_by_default);
}

#[test]
fn default_profile_hides_high_maintainability_signals() {
    let finding = finding(
        "architecture.large-file",
        FindingCategory::Architecture,
        Severity::High,
    );

    let decision = classify_visibility(&finding);

    assert_eq!(decision.intent, FindingIntent::Maintainability);
    assert!(!decision.visible_by_default);
}

#[test]
fn default_profile_hides_deep_relative_imports() {
    let finding = finding(
        "architecture.deep-relative-imports",
        FindingCategory::Architecture,
        Severity::High,
    );

    let decision = classify_visibility(&finding);

    assert_eq!(decision.intent, FindingIntent::Maintainability);
    assert!(!decision.visible_by_default);
}

#[test]
fn default_profile_keeps_high_priority_maintainability_signals() {
    let finding = finding_with_priority(
        "code-quality.long-function",
        FindingCategory::CodeQuality,
        Severity::Medium,
        RiskPriority::P1,
    );

    let decision = classify_visibility(&finding);

    assert_eq!(decision.intent, FindingIntent::Maintainability);
    assert!(decision.visible_by_default);
}

#[test]
fn default_profile_keeps_stable_import_graph_architecture_risks() {
    let finding = finding_with_priority(
        "architecture.excessive-fan-out",
        FindingCategory::Architecture,
        Severity::Medium,
        RiskPriority::P2,
    );

    let decision = classify_visibility(&finding);

    assert_eq!(decision.intent, FindingIntent::ActionableRisk);
    assert!(decision.visible_by_default);
}

#[test]
fn default_profile_keeps_validated_secret_candidates() {
    let finding = finding(
        "security.secret-candidate",
        FindingCategory::Security,
        Severity::High,
    );

    let decision = classify_visibility(&finding);

    assert_eq!(decision.intent, FindingIntent::SecurityRisk);
    assert!(decision.visible_by_default);
}

#[test]
fn default_profile_hides_script_process_exit() {
    let finding = finding_with_path(
        "language.javascript.runtime-exit-risk",
        FindingCategory::CodeQuality,
        Severity::High,
        "scripts/verify-release.mjs",
    );

    let decision = classify_visibility(&finding);

    assert_eq!(decision.intent, FindingIntent::RuntimeRisk);
    assert!(!decision.visible_by_default);
}

#[test]
fn default_profile_hides_windows_script_process_exit() {
    let finding = finding_with_path(
        "language.javascript.runtime-exit-risk",
        FindingCategory::CodeQuality,
        Severity::High,
        r"tools\verify-release.mjs",
    );

    let decision = classify_visibility(&finding);

    assert_eq!(decision.intent, FindingIntent::RuntimeRisk);
    assert!(!decision.visible_by_default);
}

#[test]
fn default_profile_keeps_source_process_exit() {
    let finding = finding_with_path(
        "language.javascript.runtime-exit-risk",
        FindingCategory::CodeQuality,
        Severity::High,
        "src/runtime/server.ts",
    );

    let decision = classify_visibility(&finding);

    assert_eq!(decision.intent, FindingIntent::RuntimeRisk);
    assert!(decision.visible_by_default);
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
        hidden_suggestions: Vec::new(),
        non_empty_lines: 1_000,
        findings: vec![visible, hidden],
        ..Default::default()
    };

    apply_visibility_profile(&mut summary, FindingVisibilityProfile::Default);

    assert_eq!(summary.visibility_profile.as_deref(), Some("default"));
    assert_eq!(summary.raw_findings_count, 2);
    assert_eq!(summary.visible_findings_count, 1);
    assert_eq!(summary.hidden_suggestions_count, 1);
    assert_eq!(summary.raw_signal_quality.findings_total, 2);
    assert_eq!(summary.visible_signal_quality.findings_total, 1);
    assert_eq!(summary.signal_quality.findings_total, 1);
    assert_eq!(summary.hidden_suggestions.len(), 1);
    assert_eq!(summary.hidden_suggestions[0].intent, "maintainability");
    assert_eq!(
        summary.hidden_suggestions[0].rule_id,
        "architecture.large-file"
    );
    assert_eq!(summary.findings.len(), 1);
    assert_eq!(summary.findings[0].rule_id, "security.secret-candidate");
}

#[test]
fn strict_profile_keeps_all_findings_and_clears_hidden_breakdown() {
    let mut summary = ScanSummary {
        non_empty_lines: 1_000,
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
    };

    apply_visibility_profile(&mut summary, FindingVisibilityProfile::Strict);

    assert_eq!(summary.visibility_profile.as_deref(), Some("strict"));
    assert_eq!(summary.raw_findings_count, 2);
    assert_eq!(summary.visible_findings_count, 2);
    assert_eq!(summary.hidden_suggestions_count, 0);
    assert!(summary.hidden_suggestions.is_empty());
    assert_eq!(summary.findings.len(), 2);
}
