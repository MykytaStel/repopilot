use serde_json::Value;
use std::fs;
use std::process::{Command, Output};
use tempfile::TempDir;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

fn run_repopilot(args: &[&str]) -> Output {
    repopilot().args(args).output().expect("run repopilot")
}

fn fixture_project() -> TempDir {
    let dir = tempfile::tempdir().expect("create fixture project");
    fs::create_dir_all(dir.path().join("src")).expect("create src dir");
    fs::write(
        dir.path().join("src/lib.rs"),
        "pub fn answer() -> i32 { 42 }\n",
    )
    .expect("write source file");
    dir
}

#[test]
fn given_conflicting_fail_flags_when_scan_runs_then_usage_error_is_returned() {
    // Given
    let project = fixture_project();
    let path = project.path().to_string_lossy().to_string();

    // When
    let output = run_repopilot(&[
        "scan",
        &path,
        "--fail-on",
        "high",
        "--fail-on-priority",
        "p1",
    ]);

    // Then
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("`--fail-on` and `--fail-on-priority` cannot be used together"),
        "stderr:\n{stderr}"
    );
}

#[test]
fn given_changed_and_since_flags_when_scan_runs_then_usage_error_is_returned() {
    // Given
    let project = fixture_project();
    let path = project.path().to_string_lossy().to_string();

    // When
    let output = run_repopilot(&["scan", &path, "--changed", "--since", "HEAD~1"]);

    // Then
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("`--changed` and `--since` cannot be used together"),
        "stderr:\n{stderr}"
    );
}

#[test]
fn given_workspace_and_changed_flags_when_scan_runs_then_usage_error_is_returned() {
    // Given
    let project = fixture_project();
    let path = project.path().to_string_lossy().to_string();

    // When
    let output = run_repopilot(&["scan", &path, "--workspace", "--changed"]);

    // Then
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("`--workspace` cannot be used with changed scans"),
        "stderr:\n{stderr}"
    );
}

#[test]
fn given_small_project_when_scan_runs_with_json_output_then_report_is_written() {
    // Given
    let project = fixture_project();
    let output_path = project.path().join("scan.json");
    let project_path = project.path().to_string_lossy().to_string();
    let output_arg = output_path.to_string_lossy().to_string();

    // When
    let output = run_repopilot(&[
        "scan",
        &project_path,
        "--format",
        "json",
        "--output",
        &output_arg,
    ]);

    // Then
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let report: Value = serde_json::from_str(
        &fs::read_to_string(&output_path).expect("scan report should be written"),
    )
    .expect("scan report should be valid JSON");
    assert!(report["files_analyzed"].as_u64().unwrap_or_default() >= 1);
}
