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

fn fixture_project_with_repeated_findings() -> TempDir {
    let dir = tempfile::tempdir().expect("create findings fixture project");
    fs::create_dir_all(dir.path().join("src")).expect("create src dir");
    fs::write(
        dir.path().join("src/lib.rs"),
        "pub fn answer() -> i32 { 42 }\n// TODO: first tracked issue\n// TODO: second tracked issue\n// TODO: third tracked issue\n",
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

#[test]
fn given_clean_repo_when_scan_runs_default_then_compact_clean_summary_is_printed() {
    // Given
    let project = fixture_project();

    // When
    let output = repopilot()
        .args(["scan", "."])
        .current_dir(project.path())
        .output()
        .expect("run repopilot");

    // Then
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("RepoPilot Scan"));
    assert!(stdout.contains("Status: Clean"));
    assert!(stdout.contains("Risk: Low"));
    assert!(stdout.contains("Health: 100/100"));
    assert!(stdout.contains("Profile: default"));
    assert!(stdout.contains("Path: ."));
    assert!(stdout.contains("Findings: 0 visible"));
    assert!(stdout.contains("Hidden suggestions:"));
    assert!(stdout.contains("No visible risks found."));
    assert!(stdout.contains("Next:"));
    assert!(!stdout.contains("Risk Summary:"));
    assert!(!stdout.contains("Signal quality:"));
    assert!(!stdout.contains("Top Rules:"));
    assert!(!stdout.contains("Scan input:"));
    assert!(!stdout.contains("Files analyzed:"));
}

#[test]
fn given_quiet_when_scan_runs_then_next_steps_are_omitted() {
    // Given
    let project = fixture_project();

    // When
    let output = repopilot()
        .args(["scan", ".", "--quiet"])
        .current_dir(project.path())
        .output()
        .expect("run repopilot");

    // Then
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("RepoPilot Scan"));
    assert!(stdout.contains("Status: Clean"));
    assert!(stdout.contains("Findings: 0 visible"));
    assert!(!stdout.contains("Next:"));
}

#[test]
fn given_no_progress_when_scan_runs_then_command_succeeds() {
    // Given
    let project = fixture_project();

    // When
    let output = repopilot()
        .args(["scan", ".", "--no-progress"])
        .current_dir(project.path())
        .output()
        .expect("run repopilot");

    // Then
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("RepoPilot Scan"));
    assert!(stdout.contains("Next:"));
}

#[test]
fn given_repo_with_visible_findings_when_scan_runs_default_then_top_findings_are_printed() {
    // Given
    let project = tempfile::tempdir().expect("create fixture project");
    fs::write(project.path().join(".env"), "API_KEY=abc123xyz987\n").expect("write env file");

    // When
    let output = repopilot()
        .args(["scan", "."])
        .current_dir(project.path())
        .output()
        .expect("run repopilot");

    // Then
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Status: Attention needed"));
    assert!(stdout.contains("Findings: 1 visible"));
    assert!(stdout.contains("Top findings:"));
    assert!(stdout.contains("security.env-file-committed"));
    assert!(stdout.contains(".env"));
    assert!(!stdout.contains("Recommendation:"));
    assert!(!stdout.contains("Evidence:"));
}

#[test]
fn given_max_findings_when_scan_runs_full_console_then_finding_details_are_limited() {
    // Given
    let project = fixture_project_with_repeated_findings();

    // When
    let output = repopilot()
        .args([
            "scan",
            ".",
            "--profile",
            "strict",
            "--output-style",
            "full",
            "--max-findings",
            "1",
        ])
        .current_dir(project.path())
        .output()
        .expect("run repopilot");

    // Then
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("showing 1 of"));
    assert!(stdout.contains("--max-findings none shows all"));
    assert_eq!(stdout.matches("Recommendation:").count(), 1);
}

#[test]
fn given_max_findings_when_scan_runs_markdown_then_finding_details_are_limited() {
    // Given
    let project = fixture_project_with_repeated_findings();

    // When
    let output = repopilot()
        .args([
            "scan",
            ".",
            "--format",
            "markdown",
            "--profile",
            "strict",
            "--max-findings",
            "1",
        ])
        .current_dir(project.path())
        .output()
        .expect("run repopilot");

    // Then
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Showing 1 of"));
    assert!(stdout.contains("Use `--max-findings none`"));
    assert_eq!(stdout.matches("  - Recommendation:").count(), 1);
}

#[test]
fn given_max_findings_when_scan_runs_json_then_machine_output_is_complete() {
    // Given
    let project = fixture_project_with_repeated_findings();

    // When
    let output = repopilot()
        .args([
            "scan",
            ".",
            "--format",
            "json",
            "--profile",
            "strict",
            "--max-findings",
            "1",
        ])
        .current_dir(project.path())
        .output()
        .expect("run repopilot");

    // Then
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).expect("json output");
    let findings = report["findings"]
        .as_array()
        .expect("findings should be an array");
    assert!(
        findings.len() > 1,
        "JSON findings should stay complete when rendered console findings are capped"
    );
}

#[test]
fn given_scan_when_output_style_full_then_diagnostic_sections_are_preserved() {
    // Given
    let project = fixture_project();

    // When
    let output = repopilot()
        .args(["scan", ".", "--output-style", "full"])
        .current_dir(project.path())
        .output()
        .expect("run repopilot");

    // Then
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Scan input:"));
    assert!(stdout.contains("Directories analyzed:"));
    assert!(stdout.contains("Non-empty lines:"));
    assert!(stdout.contains("Risk Summary:"));
    assert!(stdout.contains("Signal quality:"));
    assert!(stdout.contains("Hidden suggestions breakdown:"));
    assert!(stdout.contains("Top Risk Clusters:"));
    assert!(stdout.contains("Top Rules:"));
    assert!(stdout.contains("Languages:"));
    assert!(stdout.contains("Findings:"));
}

#[test]
fn given_no_color_when_scan_runs_then_stdout_has_no_ansi_sequences() {
    // Given
    let project = tempfile::tempdir().expect("create fixture project");
    fs::write(project.path().join(".env"), "API_KEY=abc123xyz987\n").expect("write env file");

    // When
    let forced_color = repopilot()
        .args(["scan", ".", "--color", "always"])
        .current_dir(project.path())
        .output()
        .expect("run repopilot with color");
    let no_color = repopilot()
        .args(["scan", ".", "--no-color"])
        .current_dir(project.path())
        .output()
        .expect("run repopilot without color");
    let color_never = repopilot()
        .args(["scan", ".", "--color", "never"])
        .current_dir(project.path())
        .output()
        .expect("run repopilot with color never");

    // Then
    assert!(forced_color.status.success());
    assert!(has_ansi(&forced_color.stdout));
    assert!(no_color.status.success());
    assert_no_ansi(&no_color.stdout);
    assert!(color_never.status.success());
    assert_no_ansi(&color_never.stdout);
}

#[test]
fn given_ci_or_non_tty_when_color_auto_then_output_is_plain_and_stable() {
    // Given
    let project = fixture_project();

    // When
    let output = repopilot()
        .args(["scan", ".", "--color", "auto"])
        .env("CI", "true")
        .current_dir(project.path())
        .output()
        .expect("run repopilot in CI");

    // Then
    assert!(output.status.success());
    assert_no_ansi(&output.stdout);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("RepoPilot Scan"));
    assert!(stdout.contains("Status: Clean"));
}

#[test]
fn given_machine_readable_formats_when_scan_runs_then_reports_are_unchanged() {
    // Given
    let project = tempfile::tempdir().expect("create fixture project");
    fs::write(project.path().join(".env"), "API_KEY=abc123xyz987\n").expect("write env file");

    // When / Then
    let json = scan_format(project.path(), "json");
    let json_report: Value = serde_json::from_slice(&json.stdout).expect("json output");
    assert!(json_report["findings"].as_array().is_some());
    assert_no_ansi(&json.stdout);

    let sarif = scan_format(project.path(), "sarif");
    let sarif_report: Value = serde_json::from_slice(&sarif.stdout).expect("sarif output");
    assert_eq!(sarif_report["version"], "2.1.0");
    assert_no_ansi(&sarif.stdout);

    let markdown = scan_format(project.path(), "markdown");
    let markdown_stdout = String::from_utf8_lossy(&markdown.stdout);
    assert!(markdown_stdout.contains("# RepoPilot Scan Report"));
    assert_no_ansi(&markdown.stdout);

    let html = scan_format(project.path(), "html");
    let html_stdout = String::from_utf8_lossy(&html.stdout);
    assert!(html_stdout.contains("RepoPilot Scan Report"));
    assert_no_ansi(&html.stdout);
}

#[test]
fn given_conflicting_color_flags_when_scan_runs_then_usage_error_is_returned() {
    // Given
    let project = fixture_project();

    // When
    let output = repopilot()
        .args(["scan", ".", "--color", "always", "--no-color"])
        .current_dir(project.path())
        .output()
        .expect("run repopilot");

    // Then
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("`--color always` cannot be used with `--no-color`"));
}

fn scan_format(root: &std::path::Path, format: &str) -> Output {
    let output = repopilot()
        .args([
            "scan",
            ".",
            "--format",
            format,
            "--output-style",
            "full",
            "--color",
            "always",
        ])
        .current_dir(root)
        .output()
        .expect("run repopilot scan");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn has_ansi(output: &[u8]) -> bool {
    String::from_utf8_lossy(output).contains("\u{1b}[")
}

fn assert_no_ansi(output: &[u8]) {
    assert!(
        !has_ansi(output),
        "expected no ANSI escape sequences in:\n{}",
        String::from_utf8_lossy(output)
    );
}
