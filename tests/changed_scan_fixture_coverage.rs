use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

#[test]
fn changed_scan_tracks_untracked_deleted_and_cache_paths() {
    let temp = tempdir().expect("temp dir");
    init_repo(temp.path());

    write(temp.path().join("src/live.rs"), "pub fn live() {}\n");
    write(temp.path().join("src/delete_me.rs"), "pub fn gone() {}\n");
    commit_all(temp.path(), "initial");

    fs::remove_file(temp.path().join("src/delete_me.rs")).expect("delete file");
    write(
        temp.path().join("src/untracked.rs"),
        "const API_KEY: &str = \"abc123xyz987\";\n",
    );
    write(
        temp.path().join(".repopilot/cache/noise.json"),
        "{\"should\":\"not be scanned\"}\n",
    );

    let json = scan_changed_json(temp.path(), &["--changed"]);

    assert_eq!(json["mode"], "changed");
    assert_eq!(json["repo_level_rules_included"], true);
    assert!(json["coupling_graph"].is_object());
    assert_changed_reason(&json, "deleted", 1);
    assert_changed_reason(&json, "untracked", 1);

    let changed_files = json["cache_telemetry"]["changed_files"]
        .as_array()
        .expect("changed files telemetry");

    assert!(
        changed_files.iter().all(|file| !file["path"]
            .as_str()
            .unwrap_or_default()
            .starts_with(".repopilot/cache")),
        "cache paths should not be included in changed scan telemetry: {changed_files:#?}"
    );
}

#[test]
fn local_feedback_file_suppresses_matching_finding_unless_ignored() {
    let temp = tempdir().expect("temp dir");
    init_repo(temp.path());

    write(
        temp.path().join("src/live.rs"),
        "const API_KEY: &str = \"abc123xyz987\";
",
    );
    write(
        temp.path().join("tests/live_test.rs"),
        "#[test]
fn live_smoke() { assert!(true); }
",
    );
    write(
        temp.path().join(".repopilot/feedback.yml"),
        r#"
suppressions:
  - rule_id: security.secret-candidate
    path: "src/live.rs"
    reason: fixture value accepted for this test
"#,
    );
    commit_all(temp.path(), "initial");

    let suppressed = scan_json(temp.path(), &[]);
    assert!(
        !has_rule(&suppressed, "security.secret-candidate"),
        "feedback should suppress matching security finding: {suppressed:#?}"
    );
    assert_eq!(suppressed["local_feedback"]["suppressions_loaded"], 1);
    assert_eq!(suppressed["local_feedback"]["suppressed_findings_count"], 1);
    assert_eq!(
        suppressed["local_feedback"]["unmatched_suppressions_count"],
        0
    );

    let raw = scan_json(temp.path(), &["--ignore-feedback"]);
    assert!(
        has_rule(&raw, "security.secret-candidate"),
        "ignore feedback should reveal the finding: {raw:#?}"
    );
    assert!(raw.get("local_feedback").is_none());
}

#[test]
fn malformed_feedback_file_reports_warning_and_keeps_findings() {
    let temp = tempdir().expect("temp dir");

    write(
        temp.path().join("src/live.rs"),
        "const API_KEY: &str = \"abc123xyz987\";\n",
    );
    write(
        temp.path().join(".repopilot/feedback.yml"),
        "suppressions:\n  - rule_id: [\n",
    );

    let json = scan_json(temp.path(), &[]);

    assert!(
        has_rule(&json, "security.secret-candidate"),
        "malformed feedback should not suppress findings: {json:#?}"
    );
    assert_eq!(json["local_feedback"]["suppressions_loaded"], 0);
    assert!(json["local_feedback"]["parse_error"].as_str().is_some());
    assert!(diagnostics_include(&json, "feedback.parse-failed"));
}

#[test]
fn unmatched_feedback_suppression_reports_warning() {
    let temp = tempdir().expect("temp dir");

    write(temp.path().join("src/live.rs"), "pub fn live() {}\n");
    write(
        temp.path().join(".repopilot/feedback.yml"),
        r#"
suppressions:
  - rule_id: security.secret-candidate
    path: src/live.rs
    reason: stale suppression
"#,
    );

    let json = scan_json(temp.path(), &[]);

    assert_eq!(json["local_feedback"]["suppressions_loaded"], 1);
    assert_eq!(json["local_feedback"]["suppressed_findings_count"], 0);
    assert_eq!(json["local_feedback"]["unmatched_suppressions_count"], 1);
    assert!(diagnostics_include(
        &json,
        "feedback.unmatched-suppressions"
    ));
}

#[test]
fn inspect_feedback_default_validates_without_scanning() {
    let temp = tempdir().expect("temp dir");

    write(
        temp.path().join("src/live.rs"),
        "const API_KEY: &str = \"abc123xyz987\";\n",
    );
    write(
        temp.path().join(".repopilot/feedback.yml"),
        r#"
suppressions:
  - rule_id: security.secret-candidate
    path: src/live.rs
"#,
    );
    write(temp.path().join("repopilot.toml"), "[scan");

    let output = repopilot()
        .args(["inspect", "feedback", ".", "--format", "json"])
        .current_dir(temp.path())
        .output()
        .expect("run inspect feedback");

    assert!(
        output.status.success(),
        "inspect feedback failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let json: Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(json["exists"], true);
    assert_eq!(json["suppressions_loaded"], 1);
    assert_eq!(json["invalid_suppressions_count"], 0);
    assert!(json.get("evaluation").is_none());
}

#[test]
fn inspect_feedback_evaluate_reports_applied_suppressions() {
    let temp = tempdir().expect("temp dir");

    write(
        temp.path().join("src/live.rs"),
        "const API_KEY: &str = \"abc123xyz987\";\n",
    );
    write(
        temp.path().join(".repopilot/feedback.yml"),
        r#"
suppressions:
  - rule_id: security.secret-candidate
    path: src/live.rs
"#,
    );

    let output = repopilot()
        .args(["inspect", "feedback", ".", "--evaluate", "--format", "json"])
        .current_dir(temp.path())
        .output()
        .expect("run inspect feedback");

    assert!(
        output.status.success(),
        "inspect feedback failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let json: Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(json["exists"], true);
    assert_eq!(json["suppressions_loaded"], 1);
    assert_eq!(json["evaluation"]["suppressed_findings_count"], 1);
    assert_eq!(json["evaluation"]["unmatched_suppressions_count"], 0);
}

#[test]
fn inspect_feedback_supports_console_and_markdown_formats() {
    let temp = tempdir().expect("temp dir");
    write(
        temp.path().join(".repopilot/feedback.yml"),
        "suppressions: []\n",
    );

    let console = repopilot()
        .args(["inspect", "feedback", "."])
        .current_dir(temp.path())
        .output()
        .expect("run inspect feedback console");

    assert!(
        console.status.success(),
        "console inspect failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&console.stdout),
        String::from_utf8_lossy(&console.stderr)
    );
    assert!(String::from_utf8_lossy(&console.stdout).contains("Suppressions loaded: 0"));

    let markdown = repopilot()
        .args(["inspect", "feedback", ".", "--format", "markdown"])
        .current_dir(temp.path())
        .output()
        .expect("run inspect feedback markdown");

    assert!(
        markdown.status.success(),
        "markdown inspect failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&markdown.stdout),
        String::from_utf8_lossy(&markdown.stderr)
    );
    assert!(String::from_utf8_lossy(&markdown.stdout).contains("# RepoPilot Feedback Diagnostics"));
}

#[test]
fn inspect_feedback_rejects_html_format() {
    let temp = tempdir().expect("temp dir");
    write(
        temp.path().join(".repopilot/feedback.yml"),
        "suppressions: []\n",
    );

    let output = repopilot()
        .args(["inspect", "feedback", ".", "--format", "html"])
        .current_dir(temp.path())
        .output()
        .expect("run inspect feedback");

    assert!(!output.status.success(), "html format should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid value") && stderr.contains("html"),
        "expected clear unsupported format error, got:\n{stderr}"
    );
}

fn scan_json(root: &Path, extra: &[&str]) -> Value {
    let output = repopilot()
        .args(["scan", ".", "--format", "json", "--profile", "strict"])
        .args(extra)
        .current_dir(root)
        .output()
        .expect("run scan");

    assert!(
        output.status.success(),
        "scan failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice(&output.stdout).expect("json")
}

fn scan_changed_json(root: &Path, extra: &[&str]) -> Value {
    let output = repopilot()
        .args(["scan", ".", "--format", "json"])
        .args(extra)
        .current_dir(root)
        .output()
        .expect("run changed scan");

    assert!(
        output.status.success(),
        "changed scan failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice(&output.stdout).expect("json")
}

fn has_rule(json: &Value, rule_id: &str) -> bool {
    json["findings"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|finding| finding["rule_id"] == rule_id)
}

fn diagnostics_include(json: &Value, code: &str) -> bool {
    json["diagnostics"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|diagnostic| diagnostic["code"] == code)
}

fn assert_changed_reason(json: &Value, reason: &str, count: u64) {
    assert!(
        json["cache_telemetry"]["changed_file_reasons"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|item| item["reason"] == reason && item["count"] == count),
        "expected changed reason {reason} ({count}) in {json:#?}"
    );
}

fn init_repo(root: &Path) {
    git(root, &["init"]);
    git(root, &["checkout", "-B", "main"]);
    git(root, &["config", "user.email", "repopilot@example.com"]);
    git(root, &["config", "user.name", "RepoPilot Test"]);
}

fn commit_all(root: &Path, message: &str) {
    git(root, &["add", "."]);
    git(root, &["commit", "-m", message]);
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("run git");

    assert!(
        output.status.success(),
        "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write(path: std::path::PathBuf, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, content).expect("write file");
}
