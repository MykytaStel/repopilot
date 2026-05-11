use serde_json::Value;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

fn assert_console_metric(stdout: &str, label: &str, expected_value: usize) {
    let expected_value = expected_value.to_string();

    let line = stdout
        .lines()
        .find(|line| line.trim_start().starts_with(label))
        .unwrap_or_else(|| panic!("missing console metric `{label}` in output:\n{stdout}"));

    let actual_value = line
        .split_whitespace()
        .last()
        .unwrap_or_else(|| panic!("console metric `{label}` has no value in line:\n{line}"));

    assert_eq!(
        actual_value, expected_value,
        "unexpected value for console metric `{label}` in line:\n{line}"
    );
}

#[test]
fn scan_uses_explicit_config_path_and_default_output_format() {
    let temp = tempdir().expect("failed to create temp dir");
    fs::write(temp.path().join("lib.rs"), "fn main() {}\n").expect("failed to write file");
    let config_path = temp.path().join("repopilot.toml");
    fs::write(
        &config_path,
        r#"
        [output]
        default_format = "json"
        "#,
    )
    .expect("failed to write config");

    let output = repopilot()
        .args(["scan", "."])
        .arg("--config")
        .arg(&config_path)
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot scan");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("expected JSON output");
    assert_eq!(json["files_count"], 2);
}

#[test]
fn cli_threshold_overrides_config_threshold() {
    let temp = tempdir().expect("failed to create temp dir");
    let content = (0..11)
        .map(|index| format!("fn function_{index}() {{}}"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(temp.path().join("medium.rs"), content).expect("failed to write file");
    let config_path = temp.path().join("repopilot.toml");
    fs::write(
        &config_path,
        r#"
        [architecture]
        max_file_lines = 10

        [output]
        default_format = "json"
        "#,
    )
    .expect("failed to write config");

    let output = repopilot()
        .args(["scan", ".", "--max-file-loc", "500"])
        .arg("--config")
        .arg(&config_path)
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot scan");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("expected JSON output");
    let findings = json["findings"]
        .as_array()
        .expect("findings should be array");

    assert!(findings.iter().all(|finding| {
        finding["rule_id"]
            .as_str()
            .is_none_or(|rule_id| rule_id != "architecture.large-file")
    }));
}

#[test]
fn scan_rejects_unknown_preset() {
    let temp = tempdir().expect("failed to create temp dir");
    fs::write(temp.path().join("lib.rs"), "fn main() {}\n").expect("failed to write file");

    let output = repopilot()
        .args(["scan", ".", "--preset", "strictt"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot scan");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(stderr.contains("invalid value"));
    assert!(stderr.contains("strict"));
}

#[test]
fn min_severity_recomputes_health_score_for_json_output() {
    let temp = tempdir().expect("failed to create temp dir");
    let content = (0..20)
        .map(|index| format!("pub fn function_{index}() {{}}"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(temp.path().join("large.rs"), content).expect("failed to write source file");

    let output = repopilot()
        .args([
            "scan",
            ".",
            "--format",
            "json",
            "--max-file-loc",
            "10",
            "--min-severity",
            "high",
        ])
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot scan");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("expected JSON output");
    assert_eq!(json["findings"].as_array().map(Vec::len), Some(0));
    assert_eq!(json["health_score"], 100);
}

#[test]
fn min_severity_recomputes_health_score_for_markdown_output() {
    let temp = tempdir().expect("failed to create temp dir");
    let content = (0..20)
        .map(|index| format!("pub fn function_{index}() {{}}"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(temp.path().join("large.rs"), content).expect("failed to write source file");

    let output = repopilot()
        .args([
            "scan",
            ".",
            "--format",
            "markdown",
            "--max-file-loc",
            "10",
            "--min-severity",
            "high",
        ])
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot scan");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert!(stdout.contains("- **Risk:** Clean"));
    assert!(stdout.contains("- **Health score:** 100/100"));
    assert!(stdout.contains("- **Findings:** 0 (0.0/kloc)"));
}

#[test]
fn scan_max_files_caps_analyzed_files_and_console_labels_limit() {
    let temp = tempdir().expect("failed to create temp dir");
    fs::write(temp.path().join("a.rs"), "fn a() {}\n").expect("failed to write file");
    fs::write(temp.path().join("b.rs"), "fn b() {}\n").expect("failed to write file");

    let output = repopilot()
        .args(["scan", ".", "--max-files", "1"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot scan");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert_console_metric(&stdout, "Files discovered:", 2);
    assert_console_metric(&stdout, "Files skipped (limit):", 1);
    assert_console_metric(&stdout, "Files analyzed:", 1);
    assert!(!stdout.contains("Files skipped (ignore):"));
}

#[test]
fn scan_exclude_filters_path_or_name() {
    let temp = tempdir().expect("failed to create temp dir");
    fs::write(temp.path().join("keep.rs"), "fn keep() {}\n").expect("failed to write file");
    fs::write(temp.path().join("skip.rs"), "fn skip() {}\n").expect("failed to write file");

    let output = repopilot()
        .args(["scan", ".", "--format", "json", "--exclude", "skip.rs"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot scan");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("expected JSON output");
    assert_eq!(json["files_discovered"], 1);
    assert_eq!(json["files_count"], 1);
    assert_eq!(json["coupling_graph"]["nodes"][0], "./keep.rs");
}

#[test]
fn scan_include_low_signal_restores_test_path_analysis() {
    let temp = tempdir().expect("failed to create temp dir");
    fs::create_dir(temp.path().join("tests")).expect("failed to create tests dir");
    fs::write(temp.path().join("tests/sample.rs"), "fn sample() {}\n")
        .expect("failed to write file");

    let default_output = repopilot()
        .args(["scan", ".", "--format", "json"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot scan");
    assert!(default_output.status.success());
    let default_json: Value =
        serde_json::from_slice(&default_output.stdout).expect("expected JSON output");
    assert_eq!(default_json["files_count"], 0);
    assert_eq!(default_json["files_skipped_low_signal"], 1);

    let included_output = repopilot()
        .args(["scan", ".", "--format", "json", "--include-low-signal"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot scan");
    assert!(included_output.status.success());
    let included_json: Value =
        serde_json::from_slice(&included_output.stdout).expect("expected JSON output");
    assert_eq!(included_json["files_count"], 1);
    assert_eq!(included_json["files_skipped_low_signal"], 0);
}

#[test]
fn scan_max_file_size_accepts_byte_units() {
    let temp = tempdir().expect("failed to create temp dir");
    fs::write(temp.path().join("large.rs"), "fn large() {}\n".repeat(200))
        .expect("failed to write file");

    let kb_output = repopilot()
        .args(["scan", ".", "--format", "json", "--max-file-size", "1kb"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot scan");
    assert!(kb_output.status.success());
    let kb_json: Value = serde_json::from_slice(&kb_output.stdout).expect("expected JSON output");
    assert_eq!(kb_json["files_count"], 0);
    assert_eq!(kb_json["skipped_files_count"], 1);

    for size in ["1mb", "1gb"] {
        let output = repopilot()
            .args(["scan", ".", "--format", "json", "--max-file-size", size])
            .current_dir(temp.path())
            .output()
            .expect("failed to run repopilot scan");
        assert!(output.status.success(), "{size} should be accepted");
        let json: Value = serde_json::from_slice(&output.stdout).expect("expected JSON output");
        assert_eq!(json["files_count"], 1, "{size} should not skip the file");
        assert_eq!(json["skipped_files_count"], 0);
    }
}
