use std::path::Path;

use repopilot::rules::eval::fixtures::evaluate_rule_fixtures;
use repopilot::rules::eval::{RuleEvaluationReport, RuleEvaluationRuleReport};

// Test taxonomy for this file:
//
// Layer: integration / fixture contract tests
// Scope: inspect eval-rules quality gate
// Style: BDD-style Given / When / Then
//
// Keep unit tests close to pure modules. Use this file only for end-to-end
// fixture evaluation across real fixture projects.

const RULES_WITH_013_FIXTURES: &[&str] = &[
    "security.secret-candidate",
    "security.private-key-candidate",
    "language.rust.panic-risk",
    "language.go.panic-exit-risk",
    "language.python.exception-risk",
    "language.javascript.runtime-exit-risk",
    "language.managed.fatal-exception-risk",
];

#[test]
fn given_013_rule_fixtures_when_eval_rules_runs_then_quality_gates_pass() {
    // Given
    let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/rules");

    for rule_id in RULES_WITH_013_FIXTURES {
        // When
        let report = evaluate_rule_fixtures(Some(rule_id), Some(&fixture_root))
            .unwrap_or_else(|error| panic!("failed to evaluate fixtures for {rule_id}: {error}"));

        // Then
        assert_single_rule_report(rule_id, &report);
        let rule_report = first_rule_report(rule_id, &report);
        assert_rule_fixture_coverage_is_clean(rule_id, rule_report);
    }
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
}
