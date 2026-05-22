use serde_json::Value;
use std::process::{Command, Output};

const SECRET_RULE_ID: &str = "security.secret-candidate";
const SECRET_RULE_FIXTURES_TOTAL: i64 = 4;
const SECRET_RULE_EXPECTED_FINDINGS: i64 = 3;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

fn run_repopilot(args: &[&str]) -> Output {
    repopilot().args(args).output().expect("run repopilot")
}

fn parse_json_stdout(output: &Output, context: &str) -> Value {
    assert!(
        output.status.success(),
        "{context} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "failed to parse {context} JSON: {error}\nstdout:\n{}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

#[test]
fn given_rule_catalog_when_inspect_rules_runs_then_metadata_and_filters_are_reported() {
    // Given / When
    let json = parse_json_stdout(
        &run_repopilot(&["inspect", "rules", "--format", "json"]),
        "inspect rules",
    );

    // Then
    let rules = json["rules"].as_array().expect("rules array");
    assert!(rules.iter().any(|rule| rule["rule_id"] == SECRET_RULE_ID));

    let secret_rule = rules
        .iter()
        .find(|rule| rule["rule_id"] == SECRET_RULE_ID)
        .expect("secret rule should be present");

    assert_eq!(secret_rule["semantic_source"], "text-heuristic");
    assert_eq!(secret_rule["required_scope"], "file-content");
    assert_eq!(
        secret_rule["fixture_coverage"]["fixtures_total"],
        SECRET_RULE_FIXTURES_TOTAL
    );
    assert_eq!(secret_rule["stability_gate_status"], "fixture-covered");

    // Given / When
    let preview_json = parse_json_stdout(
        &run_repopilot(&[
            "inspect",
            "rules",
            "--lifecycle",
            "preview",
            "--format",
            "json",
        ]),
        "inspect rules preview",
    );

    // Then
    assert!(
        preview_json["rules"]
            .as_array()
            .expect("preview rules")
            .iter()
            .all(|rule| rule["lifecycle"] == "preview")
    );

    // Given / When
    let source_json = parse_json_stdout(
        &run_repopilot(&[
            "inspect",
            "rules",
            "--source",
            "text-heuristic",
            "--format",
            "json",
        ]),
        "inspect rules source",
    );

    // Then
    assert!(
        source_json["rules"]
            .as_array()
            .expect("source rules")
            .iter()
            .all(|rule| rule["signal_source"] == "text-heuristic")
    );
}

#[test]
fn given_rule_id_when_inspect_rule_runs_then_known_rule_succeeds_and_unknown_rule_fails() {
    // Given / When
    let json = parse_json_stdout(
        &run_repopilot(&["inspect", "rule", SECRET_RULE_ID, "--format", "json"]),
        "inspect rule",
    );

    // Then
    assert_eq!(json["rule_id"], SECRET_RULE_ID);
    assert_eq!(json["lifecycle"], "preview");
    assert_eq!(json["false_positive_risk"], "high");

    // Given / When
    let unknown = run_repopilot(&["inspect", "rule", "missing.rule"]);

    // Then
    assert_eq!(unknown.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&unknown.stderr);
    assert!(stderr.contains("Unknown RepoPilot rule `missing.rule`"));
}

#[test]
fn given_secret_rule_fixtures_when_eval_rules_runs_then_fixture_quality_is_reported() {
    // Given / When
    let json = parse_json_stdout(
        &run_repopilot(&[
            "inspect",
            "eval-rules",
            "--rule",
            SECRET_RULE_ID,
            "--format",
            "json",
        ]),
        "inspect eval-rules",
    );

    // Then
    assert_eq!(json["rules_evaluated"], 1);
    assert_eq!(json["fixtures_total"], SECRET_RULE_FIXTURES_TOTAL);
    assert_eq!(json["expected_findings"], SECRET_RULE_EXPECTED_FINDINGS);
    assert_eq!(json["actual_findings"], SECRET_RULE_EXPECTED_FINDINGS);
    assert_eq!(json["missing_findings"], 0);
    assert_eq!(json["unexpected_findings"], 0);
    assert_eq!(json["contract_violations"], 0);
    assert_eq!(json["stable_id_failures"], 0);

    let rules = json["rules"].as_array().expect("per-rule details");
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0]["rule_id"], SECRET_RULE_ID);
    assert_eq!(rules[0]["fixtures_total"], SECRET_RULE_FIXTURES_TOTAL);
    assert_eq!(rules[0]["expected_findings"], SECRET_RULE_EXPECTED_FINDINGS);
    assert_eq!(rules[0]["actual_findings"], SECRET_RULE_EXPECTED_FINDINGS);
    assert_eq!(rules[0]["missing_findings"], 0);
    assert_eq!(rules[0]["unexpected_findings"], 0);
    assert_eq!(rules[0]["contract_violations"], 0);
    assert_eq!(rules[0]["stable_id_failures"], 0);
}
