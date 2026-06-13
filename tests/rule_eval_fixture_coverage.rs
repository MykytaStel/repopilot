use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use repopilot::rules::eval::fixtures::evaluate_rule_fixtures;
use repopilot::rules::eval::{RuleEvaluationReport, RuleEvaluationRuleReport};
use repopilot::rules::{RuleLifecycle, all_rule_metadata};
use tempfile::tempdir;

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
    "framework.django.raw-sql-query",
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

const CODE_QUALITY_RULES_WITH_FIXTURES: &[&str] = &[
    "code-quality.long-function",
    "code-quality.complex-function",
    "code-quality.complex-file",
    "code-quality.deep-control-flow",
];

// Comment-marker heuristics. All Experimental, so the quality gate does not bind
// them — the fixtures pin the line-comment matching: a marker keyword fires only
// from comment text, never from a string literal or an identifier name.
const CODE_MARKER_RULES_WITH_FIXTURES: &[&str] =
    &["code-marker.todo", "code-marker.fixme", "code-marker.hack"];

const FRAMEWORK_RULES_WITH_FIXTURES: &[&str] = &[
    "framework.react-native.deprecated-api",
    "framework.react-native.async-storage-from-core",
    "framework.react-native.direct-state-mutation",
    "framework.react-native.architecture-mismatch",
    "framework.rn-navigation-compat",
    "framework.rn-reanimated-compat",
    "framework.rn-gesture-handler-old",
];

// The convention-shaped heuristic rules (directory size, path depth, relative
// import depth, barrel re-exports, missing co-located tests). These were the
// FP-prone rules the audit flagged as unfixtured; pinning their true- and
// false-positive behaviour keeps them honest as project conventions vary.
const HEURISTIC_RULES_WITH_FIXTURES: &[&str] = &[
    "architecture.too-many-modules",
    "architecture.deep-directory-nesting",
    "architecture.deep-relative-imports",
    "architecture.barrel-file-risk",
    "testing.source-without-test",
];

// Web/JS/React/React Native framework rules. All Preview/Experimental, so the
// quality gate does not bind them — but pinning true- and false-positive
// fixtures keeps their text-/AST-lite matching honest: `var` inside an
// identifier or comment, `console.log` inside a comment or behind a sibling
// API, `prop-types` only in a TypeScript project, inline styles vs a
// `StyleSheet`, a `FlatList` with/without `keyExtractor`, and New-Arch
// (in)compatible dependencies.
const WEB_FRAMEWORK_RULES_WITH_FIXTURES: &[&str] = &[
    "framework.js.var-declaration",
    "framework.js.console-log",
    "framework.react.class-component",
    "framework.react.prop-types",
    "framework.react-native.inline-style",
    "framework.react-native.flatlist-missing-key",
    "framework.rn-new-arch-incompatible-dep",
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
fn given_code_marker_rule_fixtures_when_eval_rules_runs_then_coverage_is_clean() {
    // Given
    let fixture_root = rule_fixture_root();

    for rule_id in CODE_MARKER_RULES_WITH_FIXTURES {
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

#[test]
fn given_heuristic_rule_fixtures_when_eval_rules_runs_then_quality_gates_pass() {
    // Given
    let fixture_root = rule_fixture_root();

    for rule_id in HEURISTIC_RULES_WITH_FIXTURES {
        // When
        let report = evaluate_rule(rule_id, &fixture_root);

        // Then
        assert_single_rule_report(rule_id, &report);
        let rule_report = first_rule_report(rule_id, &report);
        assert_rule_fixture_coverage_is_clean(rule_id, rule_report);
    }
}

#[test]
fn given_web_framework_rule_fixtures_when_eval_rules_runs_then_coverage_is_clean() {
    // Given
    let fixture_root = rule_fixture_root();

    for rule_id in WEB_FRAMEWORK_RULES_WITH_FIXTURES {
        // When
        let report = evaluate_rule(rule_id, &fixture_root);

        // Then
        assert_single_rule_report(rule_id, &report);
        let rule_report = first_rule_report(rule_id, &report);
        assert_rule_fixture_coverage_is_clean(rule_id, rule_report);
    }
}

// The category tests above are a readable, well-labelled subset. This is the
// real gate: the engine autodiscovers *every* fixture directory and adds
// *every* Stable rule, so nothing can fall outside the harness by being left
// off a manual array. All Stable rules ship fixtures today, and the 16
// unfixtured rules are Preview/Experimental (which the gate does not bind), so
// this passes on a clean tree and only fails on a genuine regression: a Stable
// rule without fixtures, a promoted rule, or a broken fixture.
#[test]
fn every_fixture_dir_and_stable_rule_clears_the_quality_gate() {
    let report =
        evaluate_rule_fixtures(None, None).expect("aggregate fixture evaluation should succeed");

    let stable_count = all_rule_metadata()
        .filter(|meta| meta.lifecycle == RuleLifecycle::Stable)
        .count();
    assert!(
        report.rules_evaluated >= stable_count,
        "expected every Stable rule ({stable_count}) to be evaluated, got {}",
        report.rules_evaluated
    );

    let failing: Vec<String> = report
        .rules
        .iter()
        .filter(|rule| rule.quality_gate_failures > 0)
        .map(|rule| {
            format!(
                "  - {} ({} gate failures)",
                rule.rule_id, rule.quality_gate_failures
            )
        })
        .collect();
    assert!(
        failing.is_empty(),
        "rules failed the fixture quality gate (a Stable rule needs true- and \
false-positive fixtures, documented false-positive notes, and clean findings):\n{}",
        failing.join("\n")
    );

    assert_eq!(
        report.missing_findings, 0,
        "fixtures are missing expected findings: {report:#?}"
    );
    assert_eq!(
        report.unexpected_findings, 0,
        "fixtures produced unexpected findings: {report:#?}"
    );
    assert_eq!(
        report.contract_violations, 0,
        "fixtures produced finding contract violations: {report:#?}"
    );
    assert_eq!(
        report.stable_id_failures, 0,
        "fixtures produced unstable/duplicate finding IDs: {report:#?}"
    );

    report_preview_coverage_gap(&report);
}

// A fixture directory named after a rule that does not exist (a typo or a
// renamed rule) used to be scanned but never gated, silently contributing
// nothing. It must now be a hard error so coverage cannot rot unnoticed.
#[test]
fn fixture_directory_for_unknown_rule_is_rejected() {
    let temp = tempdir().expect("temp dir");
    let junk = temp.path().join("architecture.not-a-real-rule");
    fs::create_dir_all(&junk).expect("create junk fixture dir");
    fs::write(junk.join("expected.json"), "{\"fixtures\":[]}").expect("write expected.json");

    let error = evaluate_rule_fixtures(None, Some(temp.path()))
        .expect_err("a fixture directory with no matching rule id must be rejected");
    assert!(
        error.to_string().contains("architecture.not-a-real-rule"),
        "error should name the offending directory: {error}"
    );
}

// Coverage visibility: Preview/Experimental rules without fixtures are allowed
// (the gate only binds Stable rules), but we surface the gap so promoting a
// rule to Stable without fixtures is a deliberate, visible decision.
fn report_preview_coverage_gap(report: &RuleEvaluationReport) {
    let fixtured: BTreeSet<&str> = report
        .rules
        .iter()
        .filter(|rule| rule.fixtures_total > 0)
        .map(|rule| rule.rule_id.as_str())
        .collect();

    let mut uncovered: Vec<&str> = all_rule_metadata()
        .filter(|meta| meta.lifecycle != RuleLifecycle::Stable)
        .map(|meta| meta.rule_id)
        .filter(|rule_id| !fixtured.contains(rule_id))
        .collect();
    uncovered.sort_unstable();

    eprintln!(
        "fixture coverage: {} preview/experimental rule(s) without fixtures \
(allowed, not gated): {uncovered:?}",
        uncovered.len()
    );
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
