use serde_json::Value;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

#[test]
fn scan_accepts_sarif_format_and_writes_valid_json_to_stdout() {
    let temp = tempdir().expect("failed to create temp dir");
    fs::write(temp.path().join("lib.rs"), "fn main() {}\n").expect("failed to write file");

    let output = repopilot()
        .args(["scan", ".", "--format", "sarif"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot scan");

    assert!(output.status.success());
    let sarif: Value = serde_json::from_slice(&output.stdout).expect("expected SARIF JSON output");
    assert_eq!(sarif["version"], "2.1.0");
    assert!(sarif["runs"].is_array());
}

#[test]
fn scan_sarif_output_file_is_written_without_stdout_report() {
    let temp = tempdir().expect("failed to create temp dir");
    fs::write(temp.path().join("lib.rs"), "fn main() {}\n").expect("failed to write file");
    let output_path = temp.path().join("repopilot.sarif");

    let output = repopilot()
        .args(["scan", ".", "--format", "sarif", "--output"])
        .arg(&output_path)
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot scan");

    assert!(output.status.success());
    assert!(output.stdout.is_empty());

    let sarif = fs::read_to_string(output_path).expect("failed to read SARIF file");
    let parsed: Value = serde_json::from_str(&sarif).expect("expected SARIF file to be valid JSON");
    assert_eq!(parsed["version"], "2.1.0");
    assert!(parsed["runs"].is_array());
}

#[test]
fn scan_json_output_still_works() {
    let temp = tempdir().expect("failed to create temp dir");
    fs::write(temp.path().join("lib.rs"), "fn main() {}\n").expect("failed to write file");

    let output = repopilot()
        .args(["scan", ".", "--format", "json"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot scan");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("expected JSON output");
    assert!(json["files_analyzed"].as_u64().is_some());
}

#[test]
fn scan_default_text_output_still_works() {
    let temp = tempdir().expect("failed to create temp dir");
    fs::write(temp.path().join("lib.rs"), "fn main() {}\n").expect("failed to write file");

    let output = repopilot()
        .args(["scan", "."])
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot scan");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains("RepoPilot Scan"));
    assert!(stdout.contains("Decision: PASS"));
    assert!(stdout.contains("Findings: 0 visible"));
    assert!(!stdout.contains("Files analyzed:"));
}
