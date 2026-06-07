use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::tempdir;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

#[test]
fn snapshot_writes_default_marker_with_current_head() {
    let temp = tempdir().expect("failed to create temp dir");
    init_repo(temp.path());
    write_covered_source(temp.path(), "lib", "pub fn live() {}\n");
    commit_all(temp.path(), "initial");
    let head = git_output(temp.path(), &["rev-parse", "HEAD"]);

    let output = run_ok(temp.path(), &["snapshot"]);

    let snapshot_path = temp.path().join(".repopilot/snapshot.json");
    assert!(snapshot_path.is_file());
    let snapshot: Value =
        serde_json::from_str(&fs::read_to_string(snapshot_path).expect("failed to read snapshot"))
            .expect("snapshot should be valid JSON");
    assert_eq!(snapshot["schema_version"], 1);
    assert_eq!(snapshot["head"], head.trim());
    assert_eq!(snapshot["dirty"], false);
    assert!(snapshot["created_at"].as_str().is_some());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("RepoPilot Snapshot"));
    assert!(stdout.contains("Snapshot written to:"));
    assert!(stdout.contains("repopilot review --since-snapshot"));
}

#[test]
fn snapshot_records_dirty_worktree_before_writing_marker() {
    let temp = tempdir().expect("failed to create temp dir");
    init_repo(temp.path());
    write_covered_source(temp.path(), "lib", "pub fn live() {}\n");
    commit_all(temp.path(), "initial");
    fs::write(
        temp.path().join("src/lib.rs"),
        "pub fn live() {}\n// TODO: existing dirty work\n",
    )
    .expect("failed to dirty source");

    run_ok(temp.path(), &["snapshot"]);

    let snapshot: Value = serde_json::from_str(
        &fs::read_to_string(temp.path().join(".repopilot/snapshot.json"))
            .expect("failed to read snapshot"),
    )
    .expect("snapshot should be valid JSON");
    assert_eq!(snapshot["dirty"], true);
}

#[test]
fn review_since_snapshot_covers_committed_and_uncommitted_changes() {
    let temp = tempdir().expect("failed to create temp dir");
    init_repo(temp.path());
    write_covered_source(temp.path(), "lib", "pub fn live() {}\n");
    commit_all(temp.path(), "initial");
    run_ok(temp.path(), &["snapshot"]);

    fs::write(
        temp.path().join("src/lib.rs"),
        "pub fn live() {}\n// FIXME: committed agent change\n",
    )
    .expect("failed to write committed change");
    commit_all(temp.path(), "agent commit");
    fs::write(
        temp.path().join("src/lib.rs"),
        "pub fn live() {}\n// FIXME: committed agent change\n// TODO: uncommitted agent change\n",
    )
    .expect("failed to write uncommitted change");

    let output = run_ok(
        temp.path(),
        &[
            "review",
            "--since-snapshot",
            "--profile",
            "strict",
            "--format",
            "json",
        ],
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("review should render JSON");

    assert!(json["review"]["in_diff_findings"].as_u64().unwrap() >= 2);
    assert!(
        json["changed_files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|file| file["path"] == "src/lib.rs")
    );
    assert!(json["findings"].as_array().unwrap().iter().any(|finding| {
        finding["rule_id"] == "code-marker.fixme" && finding["in_diff"] == true
    }));
    assert!(
        json["findings"].as_array().unwrap().iter().any(|finding| {
            finding["rule_id"] == "code-marker.todo" && finding["in_diff"] == true
        })
    );
}

#[test]
fn review_since_snapshot_reports_missing_marker() {
    let temp = tempdir().expect("failed to create temp dir");
    init_repo(temp.path());
    write_covered_source(temp.path(), "lib", "pub fn live() {}\n");
    commit_all(temp.path(), "initial");

    let output = run(temp.path(), &["review", "--since-snapshot"]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("no snapshot found"));
}

#[test]
fn snapshot_requires_at_least_one_commit() {
    let temp = tempdir().expect("failed to create temp dir");
    init_repo(temp.path());

    let output = run(temp.path(), &["snapshot"]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("no commits yet"));
}

fn run(root: &Path, args: &[&str]) -> Output {
    repopilot()
        .args(args)
        .current_dir(root)
        .output()
        .expect("failed to run repopilot")
}

fn run_ok(root: &Path, args: &[&str]) -> Output {
    let output = run(root, args);
    assert!(
        output.status.success(),
        "command failed\nargs: {:?}\nstatus: {:?}\nstdout:\n{}\nstderr:\n{}",
        args,
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn write_covered_source(root: &Path, module: &str, content: &str) {
    fs::create_dir_all(root.join("src")).expect("failed to create src dir");
    fs::create_dir_all(root.join("tests")).expect("failed to create tests dir");
    fs::write(root.join(format!("src/{module}.rs")), content).expect("failed to write source");
    fs::write(
        root.join(format!("tests/{module}.rs")),
        format!("fn covers_{module}() {{}}\n"),
    )
    .expect("failed to write test");
}

fn init_repo(root: &Path) {
    git(root, &["init"]);
    git(root, &["config", "user.email", "repopilot@example.invalid"]);
    git(root, &["config", "user.name", "RepoPilot Test"]);
}

fn commit_all(root: &Path, message: &str) {
    git(root, &["add", "."]);
    git(root, &["commit", "-m", message]);
}

fn git_output(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("failed to run git");

    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("failed to run git");

    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
