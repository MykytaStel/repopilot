use serde_json::Value;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

#[test]
fn scan_writes_audit_receipt_json() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();

    fs::write(root.join(".repopilotignore"), "ignored.rs\n").expect("write repopilotignore");
    fs::write(root.join("kept.rs"), "fn kept() {}\n").expect("write kept file");
    fs::write(root.join("ignored.rs"), "fn ignored() {}\n").expect("write ignored file");

    let output = repopilot()
        .args([
            "scan",
            ".",
            "--format",
            "json",
            "--output",
            "report.json",
            "--receipt",
            "receipt.json",
        ])
        .current_dir(root)
        .output()
        .expect("run scan");

    assert!(
        output.status.success(),
        "scan should pass, stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let receipt_path = root.join("receipt.json");
    assert!(receipt_path.is_file(), "receipt file should be written");

    let receipt: Value = serde_json::from_slice(&fs::read(receipt_path).expect("read receipt"))
        .expect("valid receipt json");

    assert_eq!(receipt["schema_version"], 1);
    assert_eq!(receipt["tool"], "repopilot");
    assert!(receipt["version"].as_str().is_some());
    assert!(receipt["generated_at"].as_str().is_some());

    assert_eq!(receipt["scope"]["files_discovered"], 1);
    assert_eq!(receipt["scope"]["files_analyzed"], 1);
    assert_eq!(receipt["scope"]["files_skipped_repopilotignore"], 1);

    assert!(receipt["findings"]["total"].as_u64().is_some());
    assert!(receipt["health_score"].as_u64().is_some());
}

#[test]
fn scan_receipt_does_not_replace_regular_output() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();

    fs::write(root.join("main.rs"), "fn main() {}\n").expect("write file");

    let output = repopilot()
        .args([
            "scan",
            ".",
            "--format",
            "markdown",
            "--output",
            "report.md",
            "--receipt",
            ".repopilot/receipt.json",
        ])
        .current_dir(root)
        .output()
        .expect("run scan");

    assert!(
        output.status.success(),
        "scan should pass, stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(root.join("report.md").is_file());
    assert!(root.join(".repopilot").join("receipt.json").is_file());
}
