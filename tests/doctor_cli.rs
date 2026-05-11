use serde_json::Value;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

#[test]
fn doctor_outputs_json_with_scope_accounting() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();

    fs::write(root.join("repopilot.toml"), "").expect("write config");
    fs::write(root.join(".repopilotignore"), "ignored.rs\n").expect("write repopilotignore");
    fs::write(root.join("kept.rs"), "fn kept() {}\n").expect("write kept file");
    fs::write(root.join("ignored.rs"), "fn ignored() {}\n").expect("write ignored file");

    let output = repopilot()
        .args(["doctor", ".", "--format", "json"])
        .current_dir(root)
        .output()
        .expect("run doctor");

    assert!(
        output.status.success(),
        "doctor should pass, stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");

    assert_eq!(json["scan"]["files_discovered"], 1);
    assert_eq!(json["scan"]["files_analyzed"], 1);
    assert_eq!(json["scan"]["files_skipped_repopilotignore"], 1);

    let checks = json["checks"].as_array().expect("checks array");

    assert!(
        checks
            .iter()
            .any(|check| { check["id"] == "config" && check["status"] == "pass" })
    );

    assert!(
        checks
            .iter()
            .any(|check| { check["id"] == "repopilotignore" && check["status"] == "pass" })
    );
}

#[test]
fn doctor_console_recommends_next_command() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();

    fs::write(root.join("main.rs"), "fn main() {}\n").expect("write file");

    let output = repopilot()
        .args(["doctor", "."])
        .current_dir(root)
        .output()
        .expect("run doctor");

    assert!(
        output.status.success(),
        "doctor should pass, stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");

    assert!(stdout.contains("RepoPilot Doctor"));
    assert!(stdout.contains("Audit scope:"));
    assert!(stdout.contains("Checks:"));
    assert!(stdout.contains("Recommended next command:"));
    assert!(stdout.contains("repopilot scan ."));
}

#[test]
fn doctor_marks_empty_scope_as_failed_check() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();

    fs::create_dir(root.join("tests")).expect("create tests dir");
    fs::write(root.join("tests").join("sample.rs"), "fn sample() {}\n").expect("write test file");

    let output = repopilot()
        .args(["doctor", ".", "--format", "json"])
        .current_dir(root)
        .output()
        .expect("run doctor");

    assert!(
        output.status.success(),
        "doctor command should render diagnostics even when a check fails"
    );

    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    let checks = json["checks"].as_array().expect("checks array");

    assert!(
        checks
            .iter()
            .any(|check| { check["id"] == "scan_scope" && check["status"] == "fail" })
    );
}
