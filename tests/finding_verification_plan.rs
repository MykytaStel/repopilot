//! End-to-end proof that high-confidence findings carry a deterministic
//! verification plan through the real `scan --format json` CLI path
//! (`decision.verification_plan`), and that lower-confidence findings don't.

use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

fn scan_json(root: &Path, args: &[&str]) -> Value {
    let output = repopilot()
        .arg("scan")
        .arg(".")
        .arg("--format")
        .arg("json")
        .args(args)
        .current_dir(root)
        .output()
        .expect("failed to run scan");

    assert!(
        output.status.success(),
        "scan failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("expected JSON output")
}

fn write(path: impl AsRef<Path>, content: &str) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("failed to create parent dir");
    }
    fs::write(path, content).expect("failed to write file");
}

#[test]
fn high_confidence_finding_gets_a_verification_plan_referencing_its_evidence() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    // security.private-key-candidate is always Confidence::High (no contextual
    // downgrade), so this is a reliable high-confidence trigger.
    write(
        root.join("src/keys/id_rsa.pem"),
        "-----BEGIN RSA PRIVATE KEY-----\nMIIBOgIBAAJBAK...\n-----END RSA PRIVATE KEY-----\n",
    );

    let json = scan_json(root, &["--profile", "strict"]);
    let findings = json["findings"].as_array().expect("findings array");
    let finding = findings
        .iter()
        .find(|f| f["rule_id"] == "security.private-key-candidate")
        .expect("expected a private-key-candidate finding");

    assert_eq!(finding["confidence"], "HIGH");
    let steps = finding["decision"]["verification_plan"]["steps"]
        .as_array()
        .expect("expected a populated verification_plan.steps array");

    assert!(
        steps.iter().any(|step| step
            .as_str()
            .is_some_and(|s| s.contains("src/keys/id_rsa.pem"))),
        "expected a step referencing the evidence file: {steps:#?}"
    );
    assert!(
        steps.iter().any(|step| step
            .as_str()
            .is_some_and(|s| s.contains("does not execute code"))),
        "expected the closing what-it-cannot-verify step: {steps:#?}"
    );
}

#[test]
fn lower_confidence_finding_has_no_verification_plan() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    // A plain source-without-test finding (testing category) defaults to
    // Medium confidence with no contextual escalation trigger here.
    write(
        root.join("src/payment.rs"),
        "pub fn charge() -> bool { true }\n",
    );

    let json = scan_json(root, &["--profile", "strict"]);
    let findings = json["findings"].as_array().expect("findings array");
    let finding = findings
        .iter()
        .find(|f| f["rule_id"] == "testing.source-without-test")
        .expect("expected a source-without-test finding");

    assert_ne!(finding["confidence"], "HIGH");
    assert!(
        finding["decision"].get("verification_plan").is_none(),
        "non-high-confidence findings should not carry a verification_plan: {finding:#?}"
    );
}

#[test]
fn verification_plan_is_deterministic_across_separate_scans() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    write(
        root.join("src/keys/id_rsa.pem"),
        "-----BEGIN RSA PRIVATE KEY-----\nMIIBOgIBAAJBAK...\n-----END RSA PRIVATE KEY-----\n",
    );

    let plan_of = |json: &Value| -> Value {
        json["findings"]
            .as_array()
            .expect("findings array")
            .iter()
            .find(|f| f["rule_id"] == "security.private-key-candidate")
            .expect("expected a private-key-candidate finding")["decision"]["verification_plan"]
            .clone()
    };

    let first = scan_json(root, &["--profile", "strict"]);
    let second = scan_json(root, &["--profile", "strict"]);

    assert_eq!(plan_of(&first), plan_of(&second));
}
