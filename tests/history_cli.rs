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

#[test]
fn incompatible_history_is_explicit_and_never_looks_resolved() {
    let repo = fixture_repo();
    init_git(repo.path());
    let first = repopilot(repo.path(), &["scan", ".", "--record-history", "--quiet"]);
    assert!(first.status.success(), "stderr: {}", stderr(&first));

    corrupt_history_analysis_schema(repo.path());

    let second = repopilot(
        repo.path(),
        &["scan", ".", "--record-history", "--format", "json"],
    );
    assert!(second.status.success(), "stderr: {}", stderr(&second));
    let report: Value = serde_json::from_slice(&second.stdout).unwrap();

    assert!(report["risk_delta"].is_null());
    let diagnostics = report["diagnostics"]
        .as_array()
        .expect("history diagnostic");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0]["code"], "history.comparison-unavailable");
    assert!(
        diagnostics[0]["message"]
            .as_str()
            .unwrap()
            .contains("analysis-schema-mismatch")
    );
    assert_eq!(read_receipts(repo.path()).len(), 2);
}

#[test]
fn incompatible_history_reason_is_visible_in_default_scan_output() {
    let repo = fixture_repo();
    init_git(repo.path());
    let first = repopilot(repo.path(), &["scan", ".", "--record-history", "--quiet"]);
    assert!(first.status.success(), "stderr: {}", stderr(&first));
    corrupt_history_analysis_schema(repo.path());

    let second = repopilot(repo.path(), &["scan", ".", "--record-history"]);
    assert!(second.status.success(), "stderr: {}", stderr(&second));
    let stdout = String::from_utf8_lossy(&second.stdout);
    assert!(
        stdout.contains("history.comparison-unavailable"),
        "{stdout}"
    );
    assert!(stdout.contains("analysis-schema-mismatch"), "{stdout}");
}

#[test]
fn incompatible_history_reason_is_visible_in_review_console_and_markdown() {
    for format_args in [&[][..], &["--format", "markdown"][..]] {
        let repo = fixture_repo();
        init_git(repo.path());
        fs::write(
            repo.path().join("src/lib.rs"),
            "pub fn answer() -> i32 { 43 }\n",
        )
        .unwrap();
        let mut first_args = vec!["review", ".", "--record-history"];
        first_args.extend_from_slice(format_args);
        let first = repopilot(repo.path(), &first_args);
        assert!(first.status.success(), "stderr: {}", stderr(&first));
        corrupt_history_analysis_schema(repo.path());

        let mut second_args = vec!["review", ".", "--record-history"];
        second_args.extend_from_slice(format_args);
        let second = repopilot(repo.path(), &second_args);
        assert!(second.status.success(), "stderr: {}", stderr(&second));
        let stdout = String::from_utf8_lossy(&second.stdout);
        assert!(
            stdout.contains("history.comparison-unavailable"),
            "{stdout}"
        );
        assert!(stdout.contains("analysis-schema-mismatch"), "{stdout}");
    }
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

fn corrupt_history_analysis_schema(root: &Path) {
    let history_path = root.join(".repopilot/history/runs.jsonl");
    let mut prior: Value = serde_json::from_str(
        fs::read_to_string(&history_path)
            .unwrap()
            .lines()
            .next()
            .unwrap(),
    )
    .unwrap();
    prior["comparison"]["analysis_schema"] = Value::String("0.23".to_string());
    fs::write(
        history_path,
        format!("{}\n", serde_json::to_string(&prior).unwrap()),
    )
    .unwrap();
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}
