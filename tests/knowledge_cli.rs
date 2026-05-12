use serde_json::Value;
use std::process::Command;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

#[test]
fn knowledge_outputs_catalog_summary_as_json() {
    let output = repopilot()
        .args(["knowledge", "--format", "json"])
        .output()
        .expect("run knowledge");

    assert!(
        output.status.success(),
        "knowledge should pass, stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");

    assert!(
        json["summary"]["languages"].as_u64().unwrap_or_default() > 0,
        "expected at least one language"
    );
    assert!(
        json["summary"]["rules"].as_u64().unwrap_or_default() > 0,
        "expected at least one rule"
    );
}

#[test]
fn knowledge_can_filter_to_languages() {
    let output = repopilot()
        .args(["knowledge", "--section", "languages", "--format", "json"])
        .output()
        .expect("run knowledge languages");

    assert!(
        output.status.success(),
        "knowledge languages should pass, stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");

    assert!(json["languages"].as_array().is_some());
    assert!(json.get("rules").is_none() || json["rules"].as_array().unwrap().is_empty());

    let languages = json["languages"].as_array().expect("languages array");
    assert!(
        languages.iter().any(|language| language["id"] == "rust"),
        "expected rust language profile"
    );
}

#[test]
fn knowledge_can_filter_to_rules() {
    let output = repopilot()
        .args(["knowledge", "--section", "rules", "--format", "json"])
        .output()
        .expect("run knowledge rules");

    assert!(
        output.status.success(),
        "knowledge rules should pass, stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");

    let rules = json["rules"].as_array().expect("rules array");

    assert!(
        rules
            .iter()
            .any(|rule| rule["rule_id"] == "language.rust.panic-risk"),
        "expected rust panic risk rule"
    );
}
