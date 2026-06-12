use super::*;
use crate::findings::types::{Confidence, Evidence, FindingCategory, Severity};
use crate::risk::{RiskAssessment, RiskPriority};
use std::path::PathBuf;

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
fn default_profile_hides_low_confidence_barrel_file_risk() {
    let finding = finding(
        "architecture.barrel-file-risk",
        FindingCategory::Architecture,
        Severity::Low,
    );

    let decision = classify_visibility(&finding);

    assert_eq!(finding.confidence, Confidence::Low);
    assert_eq!(decision.intent, FindingIntent::Maintainability);
    assert!(!decision.visible_by_default);
}

#[test]
fn default_profile_hides_deep_directory_nesting() {
    let finding = finding(
        "architecture.deep-directory-nesting",
        FindingCategory::Architecture,
        Severity::Low,
    );

    let decision = classify_visibility(&finding);

    assert_eq!(decision.intent, FindingIntent::Maintainability);
    assert!(!decision.visible_by_default);
}

#[test]
fn default_profile_hides_too_many_modules_even_at_high_priority() {
    let finding = finding_with_priority(
        "architecture.too-many-modules",
        FindingCategory::Architecture,
        Severity::Medium,
        RiskPriority::P1,
    );

    let decision = classify_visibility(&finding);

    assert_eq!(decision.intent, FindingIntent::Maintainability);
    assert!(!decision.visible_by_default);
    assert_eq!(
        decision.reason,
        "experimental rules are strict-mode suggestions by default"
    );
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
fn default_profile_keeps_manifest_backed_package_boundary_violations() {
    let mut finding = finding(
        "architecture.package-boundary-violation",
        FindingCategory::Architecture,
        Severity::Medium,
    );
    finding.evidence = vec![Evidence {
        path: PathBuf::from("packages/app/src/main.ts"),
        line_start: 3,
        line_end: None,
        snippet: "imports internal file: packages/core/src/private.ts".to_string(),
    }];

    let decision = classify_visibility(&finding);

    assert_eq!(finding.confidence, Confidence::High);
    assert_eq!(decision.intent, FindingIntent::ActionableRisk);
    assert!(decision.visible_by_default);
}

#[test]
fn default_profile_hides_configured_package_boundary_violations() {
    let mut finding = finding(
        "architecture.package-boundary-violation",
        FindingCategory::Architecture,
        Severity::Medium,
    );
    finding.confidence = Confidence::Medium;
    finding.evidence = vec![Evidence {
        path: PathBuf::from("packages/app/src/main.ts"),
        line_start: 3,
        line_end: None,
        snippet: "imports internal file: packages/core/src/private.ts".to_string(),
    }];

    let decision = classify_visibility(&finding);

    assert_eq!(decision.intent, FindingIntent::Informational);
    assert!(!decision.visible_by_default);
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
fn default_profile_keeps_stable_security_findings() {
    let finding = finding(
        "security.env-file-committed",
        FindingCategory::Security,
        Severity::Critical,
    );

    let decision = classify_visibility(&finding);

    assert_eq!(decision.intent, FindingIntent::SecurityRisk);
    assert!(decision.visible_by_default);
}

#[test]
fn default_profile_hides_framework_style_rules() {
    let finding = finding(
        "framework.react-native.old-architecture",
        FindingCategory::Framework,
        Severity::Medium,
    );

    let decision = classify_visibility(&finding);

    assert_eq!(decision.intent, FindingIntent::Maintainability);
    assert!(!decision.visible_by_default);
}

#[test]
fn default_profile_keeps_manifest_backed_framework_risks() {
    let finding = finding(
        "framework.react-native.old-react-navigation",
        FindingCategory::Framework,
        Severity::Medium,
    );

    let decision = classify_visibility(&finding);

    assert_eq!(decision.intent, FindingIntent::ActionableRisk);
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
