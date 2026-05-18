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
    assert_eq!(json["repo_level_rules_included"], false);
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
    path: src/live.rs
    reason: fixture value accepted for this test
"#,
    );
    commit_all(temp.path(), "initial");

    let suppressed = scan_json(temp.path(), &[]);
    assert!(
        !has_rule(&suppressed, "security.secret-candidate"),
        "feedback should suppress matching security finding: {suppressed:#?}"
    );

    let raw = scan_json(temp.path(), &["--ignore-feedback"]);
    assert!(
        has_rule(&raw, "security.secret-candidate"),
        "ignore feedback should reveal the finding: {raw:#?}"
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
