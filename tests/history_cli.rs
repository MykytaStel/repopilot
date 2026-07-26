use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn repopilot(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
        .current_dir(root)
        .args(args)
        .output()
        .expect("run repopilot")
}

#[test]
fn scan_does_not_create_history_without_opt_in() {
    let repo = fixture_repo();
    let output = repopilot(repo.path(), &["scan", ".", "--quiet"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(!repo.path().join(".repopilot/history").exists());
}

#[test]
fn scan_record_history_writes_workspace_rooted_versioned_receipt() {
    let repo = fixture_repo();
    init_git(repo.path());
    fs::create_dir_all(repo.path().join("src/nested")).unwrap();
    let output = repopilot(
        &repo.path().join("src/nested"),
        &["scan", ".", "--quiet", "--record-history"],
    );
    assert!(output.status.success(), "stderr: {}", stderr(&output));

    let receipts = read_receipts(repo.path());
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0]["schema_version"], 1);
    assert_eq!(receipts[0]["comparison"]["scope"], "full");
    assert_eq!(receipts[0]["comparison"]["analysis_target"], "src/nested");
    assert!(!repo.path().join("src/nested/.repopilot/history").exists());
}

#[test]
fn review_record_history_uses_review_scope() {
    let repo = fixture_repo();
    init_git(repo.path());
    fs::write(
        repo.path().join("src/lib.rs"),
        "pub fn answer() -> i32 { 43 }\n",
    )
    .unwrap();
    let output = repopilot(
        repo.path(),
        &["review", ".", "--record-history", "--format", "json"],
    );
    assert!(output.status.success(), "stderr: {}", stderr(&output));

    let receipts = read_receipts(repo.path());
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0]["comparison"]["scope"], "review-changed");
}

#[test]
fn second_recorded_scan_projects_a_compatible_risk_delta() {
    let repo = fixture_repo();
    init_git(repo.path());
    let first = repopilot(
        repo.path(),
        &["scan", ".", "--record-history", "--format", "json"],
    );
    assert!(first.status.success(), "stderr: {}", stderr(&first));

    let second = repopilot(
        repo.path(),
        &["scan", ".", "--record-history", "--format", "json"],
    );
    assert!(second.status.success(), "stderr: {}", stderr(&second));
    let report: Value = serde_json::from_slice(&second.stdout).unwrap();

    assert!(report["risk_delta"].is_object());
    assert!(report["risk_delta"]["new_findings"].is_array());
    assert!(report["risk_delta"]["persisting_findings"].is_array());
    assert!(report["risk_delta"]["resolved_findings"].is_array());
}

fn fixture_repo() -> tempfile::TempDir {
    let repo = tempfile::tempdir().unwrap();
    fs::create_dir_all(repo.path().join("src")).unwrap();
    fs::write(
        repo.path().join("src/lib.rs"),
        "pub fn answer() -> i32 { 42 }\n",
    )
    .unwrap();
    repo
}

fn init_git(root: &Path) {
    for args in [
        &["init", "-q"][..],
        &["config", "user.email", "test@example.com"][..],
        &["config", "user.name", "Test"][..],
        &["add", "."][..],
        &["commit", "-qm", "initial"][..],
    ] {
        let output = Command::new("git")
            .current_dir(root)
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success(), "git stderr: {}", stderr(&output));
    }
}

fn read_receipts(root: &Path) -> Vec<Value> {
    fs::read_to_string(root.join(".repopilot/history/runs.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}
