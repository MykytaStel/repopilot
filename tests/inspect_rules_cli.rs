use serde_json::Value;
use std::process::Command;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

#[test]
fn inspect_rules_lists_and_filters_catalog() {
    let output = repopilot()
        .args(["inspect", "rules", "--format", "json"])
        .output()
        .expect("inspect rules");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("rules json");
    assert!(
        json["rules"]
            .as_array()
            .expect("rules array")
            .iter()
            .any(|rule| rule["rule_id"] == "security.secret-candidate")
    );
    let secret_rule = json["rules"]
        .as_array()
        .expect("rules array")
        .iter()
        .find(|rule| rule["rule_id"] == "security.secret-candidate")
        .expect("secret rule should be present");
    assert_eq!(secret_rule["semantic_source"], "text-heuristic");
    assert_eq!(secret_rule["required_scope"], "file-content");
    assert_eq!(secret_rule["fixture_coverage"]["fixtures_total"], 2);
    assert_eq!(secret_rule["stability_gate_status"], "fixture-covered");

    let preview = repopilot()
        .args([
            "inspect",
            "rules",
            "--lifecycle",
            "preview",
            "--format",
            "json",
        ])
        .output()
        .expect("inspect rules preview");
    assert!(preview.status.success());
    let preview_json: Value = serde_json::from_slice(&preview.stdout).expect("preview json");
    assert!(
        preview_json["rules"]
            .as_array()
            .expect("preview rules")
            .iter()
            .all(|rule| rule["lifecycle"] == "preview")
    );

    let source = repopilot()
        .args([
            "inspect",
            "rules",
            "--source",
            "text-heuristic",
            "--format",
            "json",
        ])
        .output()
        .expect("inspect rules source");
    assert!(source.status.success());
    let source_json: Value = serde_json::from_slice(&source.stdout).expect("source json");
    assert!(
        source_json["rules"]
            .as_array()
            .expect("source rules")
            .iter()
            .all(|rule| rule["signal_source"] == "text-heuristic")
    );
}

#[test]
fn inspect_rule_returns_known_rule_and_rejects_unknown_rule() {
    let known = repopilot()
        .args([
            "inspect",
            "rule",
            "security.secret-candidate",
            "--format",
            "json",
        ])
        .output()
        .expect("inspect rule");
    assert!(known.status.success());
    let json: Value = serde_json::from_slice(&known.stdout).expect("rule json");
    assert_eq!(json["rule_id"], "security.secret-candidate");
    assert_eq!(json["lifecycle"], "preview");
    assert_eq!(json["false_positive_risk"], "high");

    let unknown = repopilot()
        .args(["inspect", "rule", "missing.rule"])
        .output()
        .expect("inspect unknown rule");
    assert_eq!(unknown.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&unknown.stderr);
    assert!(stderr.contains("Unknown RepoPilot rule `missing.rule`"));
}

#[test]
fn inspect_eval_rules_reports_fixture_quality() {
    let output = repopilot()
        .args([
            "inspect",
            "eval-rules",
            "--rule",
            "security.secret-candidate",
            "--format",
            "json",
        ])
        .output()
        .expect("eval rules");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("eval json");
    assert_eq!(json["rules_evaluated"], 1);
    assert_eq!(json["fixtures_total"], 2);
    assert_eq!(json["expected_findings"], 1);
    assert_eq!(json["actual_findings"], 1);
    assert_eq!(json["missing_findings"], 0);
    assert_eq!(json["unexpected_findings"], 0);
    assert_eq!(json["contract_violations"], 0);
    assert_eq!(json["stable_id_failures"], 0);
    let rules = json["rules"].as_array().expect("per-rule details");
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0]["rule_id"], "security.secret-candidate");
    assert_eq!(rules[0]["fixtures_total"], 2);
    assert_eq!(rules[0]["expected_findings"], 1);
    assert_eq!(rules[0]["actual_findings"], 1);
    assert_eq!(rules[0]["missing_findings"], 0);
    assert_eq!(rules[0]["unexpected_findings"], 0);
    assert_eq!(rules[0]["contract_violations"], 0);
    assert_eq!(rules[0]["stable_id_failures"], 0);
}
