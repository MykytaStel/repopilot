use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

#[test]
fn review_reports_working_tree_findings_on_changed_lines() {
    let temp = tempdir().expect("failed to create temp dir");
    init_repo(temp.path());
    write_covered_source(temp.path(), "lib", "pub fn live() {}\n");
    commit_all(temp.path(), "initial");

    fs::write(
        temp.path().join("src/lib.rs"),
        "pub fn live() {}\n// TODO: review this\n",
    )
    .expect("failed to modify source file");

    let json = run_review_json(temp.path(), &["review", ".", "--format", "json"]);

    assert_eq!(json["review"]["in_diff_findings"], 1);
    assert_eq!(json["review"]["out_of_diff_findings"], 0);
    assert_eq!(json["changed_files"][0]["path"], "src/lib.rs");
    assert!(
        json["findings"].as_array().unwrap().iter().any(|finding| {
            finding["rule_id"] == "code-marker.todo" && finding["in_diff"] == true
        })
    );
}

#[test]
fn review_treats_untracked_files_as_fully_changed() {
    let temp = tempdir().expect("failed to create temp dir");
    init_repo(temp.path());
    fs::create_dir_all(temp.path().join("src")).expect("failed to create src dir");
    fs::create_dir_all(temp.path().join("tests")).expect("failed to create tests dir");
    fs::write(temp.path().join("src/lib.rs"), "pub fn live() {}\n").expect("failed to write lib");
    fs::write(temp.path().join("tests/lib.rs"), "fn covers_lib() {}\n")
        .expect("failed to write test");
    commit_all(temp.path(), "initial");

    write_covered_source(
        temp.path(),
        "creds",
        "const API_KEY: &str = \"abc12345\";\n",
    );

    let json = run_review_json(temp.path(), &["review", ".", "--format", "json"]);

    assert!(
        json["changed_files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|file| file["path"] == "src/creds.rs" && file["status"] == "untracked")
    );
    assert!(json["findings"].as_array().unwrap().iter().any(|finding| {
        finding["rule_id"] == "security.secret-candidate" && finding["in_diff"] == true
    }));
}

#[test]
fn review_accepts_file_paths() {
    let temp = tempdir().expect("failed to create temp dir");
    init_repo(temp.path());
    write_covered_source(temp.path(), "lib", "pub fn live() {}\n");
    commit_all(temp.path(), "initial");

    fs::write(
        temp.path().join("src/lib.rs"),
        "pub fn live() {}\n// TODO: review file path\n",
    )
    .expect("failed to modify source file");

    let json = run_review_json(temp.path(), &["review", "src/lib.rs", "--format", "json"]);

    assert_eq!(json["review"]["in_diff_findings"], 1);
    assert_eq!(json["changed_files"][0]["path"], "src/lib.rs");
}

#[test]
fn review_supports_base_head_refs() {
    let temp = tempdir().expect("failed to create temp dir");
    init_repo(temp.path());
    write_covered_source(temp.path(), "lib", "pub fn live() {}\n");
    commit_all(temp.path(), "initial");
    let base = git_output(temp.path(), &["rev-parse", "HEAD"]);

    fs::write(
        temp.path().join("src/lib.rs"),
        "pub fn live() {}\n// FIXME: changed on branch\n",
    )
    .expect("failed to modify source file");
    commit_all(temp.path(), "add fixme");

    let json = run_review_json(
        temp.path(),
        &["review", ".", "--base", base.trim(), "--format", "json"],
    );

    assert_eq!(json["review"]["in_diff_findings"], 1);
    assert!(json["findings"].as_array().unwrap().iter().any(|finding| {
        finding["rule_id"] == "code-marker.fixme" && finding["in_diff"] == true
    }));
}

#[test]
fn review_fail_on_new_high_ignores_out_of_diff_high_findings() {
    let temp = tempdir().expect("failed to create temp dir");
    init_repo(temp.path());
    write_covered_source(
        temp.path(),
        "config",
        "const API_KEY: &str = \"abc12345\";\n",
    );
    write_covered_source(temp.path(), "lib", "pub fn live() {}\n");
    commit_all(temp.path(), "initial");

    fs::write(
        temp.path().join("src/lib.rs"),
        "pub fn live() {}\n// TODO: changed low finding\n",
    )
    .expect("failed to modify source file");

    let output = repopilot()
        .args(["review", ".", "--fail-on", "new-high"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run review");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("CI gate: passed (new-high)"));
    assert!(stdout.contains("Out-of-diff findings: 1"));
}

#[test]
fn review_fail_on_new_high_uses_baseline_status_for_in_diff_findings() {
    let temp = tempdir().expect("failed to create temp dir");
    init_repo(temp.path());
    write_covered_source(
        temp.path(),
        "config",
        "const API_KEY: &str = \"abc12345\";\n",
    );
    commit_all(temp.path(), "initial");
    create_baseline(temp.path());
    commit_all(temp.path(), "baseline");

    fs::write(
        temp.path().join("src/config.rs"),
        "const API_KEY: &str = \"def67890\";\n",
    )
    .expect("failed to modify existing secret");

    let passing = repopilot()
        .args([
            "review",
            ".",
            "--baseline",
            ".repopilot/baseline.json",
            "--fail-on",
            "new-high",
        ])
        .current_dir(temp.path())
        .output()
        .expect("failed to run review");

    assert!(passing.status.success());
    assert!(String::from_utf8_lossy(&passing.stdout).contains("CI gate: passed (new-high)"));

    fs::write(
        temp.path().join("src/config.rs"),
        "const API_KEY: &str = \"def67890\";\nconst ACCESS_TOKEN: &str = \"abc12345\";\n",
    )
    .expect("failed to add new secret");

    let failing = repopilot()
        .args([
            "review",
            ".",
            "--baseline",
            ".repopilot/baseline.json",
            "--fail-on",
            "new-high",
        ])
        .current_dir(temp.path())
        .output()
        .expect("failed to run review");

    assert!(!failing.status.success());
    assert!(String::from_utf8_lossy(&failing.stdout).contains("CI gate: failed (new-high)"));
    assert!(String::from_utf8_lossy(&failing.stderr).contains("RepoPilot CI Gate failed"));
}

#[test]
fn review_rejects_head_without_base() {
    let temp = tempdir().expect("failed to create temp dir");
    init_repo(temp.path());
    write_covered_source(temp.path(), "lib", "pub fn live() {}\n");
    commit_all(temp.path(), "initial");

    let output = repopilot()
        .args(["review", ".", "--head", "HEAD"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run review");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--head` requires --base"));
}

fn run_review_json(root: &Path, args: &[&str]) -> Value {
    let output = repopilot()
        .args(args)
        .current_dir(root)
        .output()
        .expect("failed to run review");

    assert!(output.status.success());
    serde_json::from_slice(&output.stdout).expect("expected JSON output")
}

fn create_baseline(root: &Path) {
    let output = repopilot()
        .args(["baseline", "create", "."])
        .current_dir(root)
        .output()
        .expect("failed to run baseline create");
    assert!(output.status.success());
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

    assert!(output.status.success());
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
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}
