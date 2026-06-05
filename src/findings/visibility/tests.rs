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
fn default_profile_hides_high_priority_broad_maintainability_heuristics() {
    let finding = finding_with_priority(
        "code-quality.long-function",
        FindingCategory::CodeQuality,
        Severity::Medium,
        RiskPriority::P1,
    );

    let decision = classify_visibility(&finding);

    assert_eq!(decision.intent, FindingIntent::Maintainability);
    assert!(!decision.visible_by_default);
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
