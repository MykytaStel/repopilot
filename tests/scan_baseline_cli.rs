use serde_json::Value;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

#[test]
fn scan_with_baseline_marks_matching_finding_as_existing() {
    let temp = tempdir().expect("failed to create temp dir");
    write_project_with_secret(temp.path(), "config");
    create_baseline(temp.path());

    let output = repopilot()
        .args([
            "scan",
            ".",
            "--baseline",
            ".repopilot/baseline.json",
            "--format",
            "json",
        ])
        .current_dir(temp.path())
        .output()
        .expect("failed to run scan");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("expected JSON output");

    assert_eq!(json["baseline"]["new_findings"], 0);
    assert_eq!(json["baseline"]["existing_findings"], 1);
    assert_eq!(json["findings"][0]["baseline_status"], "existing");
}

#[test]
fn scan_with_baseline_marks_new_finding_as_new_and_shows_counts() {
    let temp = tempdir().expect("failed to create temp dir");
    write_project_with_secret(temp.path(), "config");
    create_baseline(temp.path());
    write_project_with_secret(temp.path(), "creds");

    let json_output = repopilot()
        .args([
            "scan",
            ".",
            "--baseline",
            ".repopilot/baseline.json",
            "--format",
            "json",
        ])
        .current_dir(temp.path())
        .output()
        .expect("failed to run scan");

    assert!(json_output.status.success());
    let json: Value = serde_json::from_slice(&json_output.stdout).expect("expected JSON output");

    assert_eq!(json["baseline"]["new_findings"], 1);
    assert_eq!(json["baseline"]["existing_findings"], 1);
    assert!(
        json["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| finding["baseline_status"] == "new")
    );

    let console_output = repopilot()
        .args(["scan", ".", "--baseline", ".repopilot/baseline.json"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run scan");

    assert!(console_output.status.success());
    let stdout = String::from_utf8_lossy(&console_output.stdout);
    assert!(stdout.contains("Baseline: .repopilot/baseline.json"));
    assert!(stdout.contains("New findings: 1"));
    assert!(stdout.contains("Existing findings: 1"));
}

#[test]
fn scan_with_baseline_markdown_includes_status_counts() {
    let temp = tempdir().expect("failed to create temp dir");
    write_project_with_secret(temp.path(), "config");
    create_baseline(temp.path());
    write_project_with_secret(temp.path(), "creds");

    let output = repopilot()
        .args([
            "scan",
            ".",
            "--baseline",
            ".repopilot/baseline.json",
            "--format",
            "markdown",
        ])
        .current_dir(temp.path())
        .output()
        .expect("failed to run scan");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("- **New findings:** 1"));
    assert!(stdout.contains("- **Existing findings:** 1"));
    assert!(stdout.contains("| Severity | Baseline | Rule | Title | Evidence |"));
    assert!(stdout.contains("| HIGH | existing | `security.secret-candidate`"));
    assert!(stdout.contains("| HIGH | new | `security.secret-candidate`"));
}

#[test]
fn scan_with_baseline_html_includes_status_counts() {
    let temp = tempdir().expect("failed to create temp dir");
    write_project_with_secret(temp.path(), "config");
    create_baseline(temp.path());
    write_project_with_secret(temp.path(), "creds");

    let output = repopilot()
        .args([
            "scan",
            ".",
            "--baseline",
            ".repopilot/baseline.json",
            "--format",
            "html",
        ])
        .current_dir(temp.path())
        .output()
        .expect("failed to run scan");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Baseline: <code>.repopilot/baseline.json</code>"));
    assert!(stdout.contains("<div class=\"num\">1</div><div class=\"label\">New</div>"));
    assert!(stdout.contains("<div class=\"num\">1</div><div class=\"label\">Existing</div>"));
    assert!(stdout.contains("<th>Baseline</th>"));
    assert!(stdout.contains("<span class=\"status existing\">existing</span>"));
    assert!(stdout.contains("<span class=\"status new\">new</span>"));
}

#[test]
fn scan_with_baseline_sarif_includes_baseline_properties() {
    let temp = tempdir().expect("failed to create temp dir");
    write_project_with_secret(temp.path(), "config");
    create_baseline(temp.path());
    write_project_with_secret(temp.path(), "creds");

    let output = repopilot()
        .args([
            "scan",
            ".",
            "--baseline",
            ".repopilot/baseline.json",
            "--format",
            "sarif",
        ])
        .current_dir(temp.path())
        .output()
        .expect("failed to run scan");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("expected SARIF output");
    let results = json["runs"][0]["results"]
        .as_array()
        .expect("SARIF results should be an array");

    assert!(results.iter().any(|result| {
        result["properties"]["baselineStatus"] == "existing"
            && result["properties"]["baselineKey"].as_str().is_some()
    }));
    assert!(results.iter().any(|result| {
        result["properties"]["baselineStatus"] == "new"
            && result["properties"]["baselineKey"].as_str().is_some()
    }));
}

#[test]
fn fail_on_new_high_passes_when_no_new_high_findings_exist() {
    let temp = tempdir().expect("failed to create temp dir");
    write_project_with_secret(temp.path(), "config");
    create_baseline(temp.path());

    let output = repopilot()
        .args([
            "scan",
            ".",
            "--baseline",
            ".repopilot/baseline.json",
            "--fail-on",
            "new-high",
        ])
        .current_dir(temp.path())
        .output()
        .expect("failed to run scan");

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("CI gate: passed (new-high)"));
}

#[test]
fn fail_on_new_high_fails_when_new_high_finding_exists() {
    let temp = tempdir().expect("failed to create temp dir");
    write_project_with_secret(temp.path(), "config");
    create_baseline(temp.path());
    write_project_with_secret(temp.path(), "creds");

    let output = repopilot()
        .args([
            "scan",
            ".",
            "--baseline",
            ".repopilot/baseline.json",
            "--fail-on",
            "new-high",
        ])
        .current_dir(temp.path())
        .output()
        .expect("failed to run scan");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("CI gate: failed (new-high)"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("RepoPilot CI Gate failed"));
}

#[test]
fn lower_severity_new_findings_do_not_fail_higher_threshold() {
    let temp = tempdir().expect("failed to create temp dir");
    fs::create_dir_all(temp.path().join("src")).expect("failed to create src dir");
    fs::create_dir_all(temp.path().join("tests")).expect("failed to create tests dir");
    fs::write(
        temp.path().join("src/lib.rs"),
        "pub fn live() {}\n// TODO: follow up\n",
    )
    .expect("failed to write source file");
    fs::write(temp.path().join("tests/lib.rs"), "fn covers_lib() {}\n")
        .expect("failed to write test file");

    let output = repopilot()
        .args(["scan", ".", "--fail-on", "new-high"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run scan");

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("New findings: 1"));
}

#[test]
fn fail_on_without_baseline_treats_all_findings_as_new() {
    let temp = tempdir().expect("failed to create temp dir");
    write_project_with_secret(temp.path(), "config");

    let output = repopilot()
        .args(["scan", ".", "--fail-on", "new-high"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run scan");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("Baseline: none (all findings treated as new)")
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("RepoPilot CI Gate failed"));
}

fn create_baseline(root: &std::path::Path) {
    let output = repopilot()
        .args(["baseline", "create", "."])
        .current_dir(root)
        .output()
        .expect("failed to run baseline create");
    assert!(output.status.success());
}

fn write_project_with_secret(root: &std::path::Path, module: &str) {
    fs::create_dir_all(root.join("src")).expect("failed to create src dir");
    fs::create_dir_all(root.join("tests")).expect("failed to create tests dir");
    fs::write(
        root.join(format!("src/{module}.rs")),
        "const API_KEY: &str = \"abc12345\";\n",
    )
    .expect("failed to write source file");
    fs::write(
        root.join(format!("tests/{module}.rs")),
        "fn covers_module() {}\n",
    )
    .expect("failed to write test file");
}
