use std::path::Path;

use repopilot::rules::eval::fixtures::evaluate_rule_fixtures;
use repopilot::rules::eval::{RuleEvaluationReport, RuleEvaluationRuleReport};

// Test taxonomy for this file:
//
// Layer: integration / fixture contract tests
// Scope: rule quality gate (the `evaluate_rule_fixtures` engine path)
// Style: BDD-style Given / When / Then
//
// Unit tests should stay close to pure modules. This file is for end-to-end
// fixture evaluation across real fixture projects.

const SECURITY_RULES_WITH_FIXTURES: &[&str] = &[
    "security.env-file-committed",
    "security.secret-candidate",
    "security.private-key-candidate",
    "framework.django.debug-true",
    "framework.django.missing-allowed-hosts",
];

const IMPORT_GRAPH_RULES_WITH_FIXTURES: &[&str] = &[
    "architecture.circular-dependency",
    "architecture.excessive-fan-out",
    "architecture.high-instability-hub",
    "architecture.dead-module",
    "architecture.test-leak",
    "architecture.package-boundary-violation",
];

const RUNTIME_RULES_WITH_FIXTURES: &[&str] = &[
    "language.rust.panic-risk",
    "language.go.panic-exit-risk",
    "language.python.exception-risk",
    "language.javascript.runtime-exit-risk",
    "language.managed.fatal-exception-risk",
];

const CODE_QUALITY_RULES_WITH_FIXTURES: &[&str] = &["code-quality.long-function"];

const FRAMEWORK_RULES_WITH_FIXTURES: &[&str] = &[
    "framework.react-native.deprecated-api",
    "framework.react-native.async-storage-from-core",
    "framework.react-native.direct-state-mutation",
    "framework.react-native.architecture-mismatch",
    "framework.rn-navigation-compat",
    "framework.rn-reanimated-compat",
    "framework.rn-gesture-handler-old",
];

#[test]
fn given_security_rule_fixtures_when_eval_rules_runs_then_quality_gates_pass() {
    // Given
    let fixture_root = rule_fixture_root();

    for rule_id in SECURITY_RULES_WITH_FIXTURES {
        // When
        let report = evaluate_rule(rule_id, &fixture_root);

        // Then
        assert_single_rule_report(rule_id, &report);
        let rule_report = first_rule_report(rule_id, &report);
        assert_rule_fixture_coverage_is_clean(rule_id, rule_report);
    }
}

#[test]
fn given_import_graph_rule_fixtures_when_eval_rules_runs_then_quality_gates_pass() {
    // Given
    let fixture_root = rule_fixture_root();

    for rule_id in IMPORT_GRAPH_RULES_WITH_FIXTURES {
        // When
        let report = evaluate_rule(rule_id, &fixture_root);

        // Then
        assert_single_rule_report(rule_id, &report);
        let rule_report = first_rule_report(rule_id, &report);
        assert_rule_fixture_coverage_is_clean(rule_id, rule_report);
    }
}

#[test]
fn given_runtime_rule_fixtures_when_eval_rules_runs_then_quality_gates_pass() {
    // Given
    let fixture_root = rule_fixture_root();

    for rule_id in RUNTIME_RULES_WITH_FIXTURES {
        // When
        let report = evaluate_rule(rule_id, &fixture_root);

        // Then
        assert_single_rule_report(rule_id, &report);
        let rule_report = first_rule_report(rule_id, &report);
        assert_rule_fixture_coverage_is_clean(rule_id, rule_report);
    }
}

#[test]
fn given_code_quality_rule_fixtures_when_eval_rules_runs_then_quality_gates_pass() {
    // Given
    let fixture_root = rule_fixture_root();

    for rule_id in CODE_QUALITY_RULES_WITH_FIXTURES {
        // When
        let report = evaluate_rule(rule_id, &fixture_root);

        // Then
        assert_single_rule_report(rule_id, &report);
        let rule_report = first_rule_report(rule_id, &report);
        assert_rule_fixture_coverage_is_clean(rule_id, rule_report);
    }
}

#[test]
fn given_framework_rule_fixtures_when_eval_rules_runs_then_quality_gates_pass() {
    // Given
    let fixture_root = rule_fixture_root();

    for rule_id in FRAMEWORK_RULES_WITH_FIXTURES {
        // When
        let report = evaluate_rule(rule_id, &fixture_root);

        // Then
        assert_single_rule_report(rule_id, &report);
        let rule_report = first_rule_report(rule_id, &report);
        assert_rule_fixture_coverage_is_clean(rule_id, rule_report);
    }
}

fn rule_fixture_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/rules")
}

fn evaluate_rule(rule_id: &str, fixture_root: &Path) -> RuleEvaluationReport {
    evaluate_rule_fixtures(Some(rule_id), Some(fixture_root))
        .unwrap_or_else(|error| panic!("failed to evaluate fixtures for {rule_id}: {error}"))
}

fn assert_single_rule_report(rule_id: &str, report: &RuleEvaluationReport) {
    assert_eq!(
        report.rules_evaluated, 1,
        "expected exactly one evaluated rule for {rule_id}: {report:#?}"
    );
}

fn first_rule_report<'a>(
    rule_id: &str,
    report: &'a RuleEvaluationReport,
) -> &'a RuleEvaluationRuleReport {
    report
        .rules
        .first()
        .unwrap_or_else(|| panic!("missing rule report for {rule_id}"))
}

fn assert_rule_fixture_coverage_is_clean(rule_id: &str, rule_report: &RuleEvaluationRuleReport) {
    assert!(
        rule_report.fixtures_total >= 2,
        "expected true-positive and false-positive fixtures for {rule_id}: {rule_report:#?}"
    );
    assert!(
        rule_report.has_true_positive_fixture,
        "expected true-positive fixture for {rule_id}: {rule_report:#?}"
    );
    assert!(
        rule_report.has_false_positive_fixture,
        "expected false-positive fixture for {rule_id}: {rule_report:#?}"
    );
    assert_eq!(
        rule_report.missing_findings, 0,
        "fixture is missing expected findings for {rule_id}: {rule_report:#?}"
    );
    assert_eq!(
        rule_report.unexpected_findings, 0,
        "fixture has unexpected findings for {rule_id}: {rule_report:#?}"
    );
    assert_eq!(
        rule_report.contract_violations, 0,
        "fixture produced finding contract violations for {rule_id}: {rule_report:#?}"
    );
    assert_eq!(
        rule_report.stable_id_failures, 0,
        "fixture produced unstable/duplicate finding IDs for {rule_id}: {rule_report:#?}"
    );
    assert_eq!(
        rule_report.quality_gate_failures, 0,
        "fixture failed the rule quality gate for {rule_id}: {rule_report:#?}"
    );
}
