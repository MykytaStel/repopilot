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

    let json = run_review_json(
        temp.path(),
        &["review", ".", "--format", "json", "--profile", "strict"],
    );

    assert_eq!(json["schema_version"], "0.19");
    assert_eq!(json["report"]["kind"], "review");
    assert!(json["risk_summary"]["total"].as_u64().is_some());
    assert!(json["review"]["in_diff_findings"].as_u64().unwrap() >= 1);
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
fn review_json_includes_local_feedback_metadata() {
    let temp = tempdir().expect("failed to create temp dir");
    init_repo(temp.path());
    write_covered_source(temp.path(), "lib", "pub fn live() {}\n");
    commit_all(temp.path(), "initial");

    write_covered_source(
        temp.path(),
        "creds",
        "const API_KEY: &str = \"abc12345\";\n",
    );
    fs::create_dir_all(temp.path().join(".repopilot")).expect("create repopilot dir");
    fs::write(
        temp.path().join(".repopilot/feedback.yml"),
        r#"
suppressions:
  - rule_id: security.secret-candidate
    path: src/creds.rs
    reason: accepted fixture
"#,
    )
    .expect("write feedback");

    let json = run_review_json(temp.path(), &["review", ".", "--format", "json"]);

    assert_eq!(json["local_feedback"]["suppressions_loaded"], 1);
    assert_eq!(json["local_feedback"]["suppressed_findings_count"], 1);
    assert!(
        json["findings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|finding| finding["rule_id"] != "security.secret-candidate")
    );
}

#[test]
fn review_ignore_feedback_reveals_suppressed_findings() {
    let temp = tempdir().expect("failed to create temp dir");
    init_repo(temp.path());
    write_covered_source(temp.path(), "lib", "pub fn live() {}\n");
    commit_all(temp.path(), "initial");

    write_covered_source(
        temp.path(),
        "creds",
        "const API_KEY: &str = \"abc12345\";\n",
    );
    fs::create_dir_all(temp.path().join(".repopilot")).expect("create repopilot dir");
    fs::write(
        temp.path().join(".repopilot/feedback.yml"),
        r#"
suppressions:
  - rule_id: security.secret-candidate
    path: src/creds.rs
"#,
    )
    .expect("write feedback");

    let json = run_review_json(
        temp.path(),
        &["review", ".", "--format", "json", "--ignore-feedback"],
    );

    assert!(json.get("local_feedback").is_none());
    assert!(json["findings"].as_array().unwrap().iter().any(|finding| {
        finding["rule_id"] == "security.secret-candidate" && finding["in_diff"] == true
    }));
}

#[test]
fn review_min_confidence_keeps_only_high_confidence_findings() {
    let temp = tempdir().expect("failed to create temp dir");
    init_repo(temp.path());
    write_covered_source(temp.path(), "lib", "pub fn live() {}\n");
    commit_all(temp.path(), "initial");

    fs::write(
        temp.path().join("src/lib.rs"),
        "pub fn live() {}\n// TODO: medium-confidence review note\nconst API_KEY: &str = \"abc12345\";\n",
    )
    .expect("failed to modify source file");

    let json = run_review_json(
        temp.path(),
        &[
            "review",
            ".",
            "--format",
            "json",
            "--min-confidence",
            "high",
        ],
    );
    let findings = json["findings"].as_array().unwrap();

    assert!(
        findings
            .iter()
            .all(|finding| finding["confidence"] == "HIGH")
    );
    assert!(
        findings
            .iter()
            .any(|finding| finding["rule_id"] == "security.secret-candidate")
    );
    assert!(
        findings
            .iter()
            .all(|finding| finding["rule_id"] != "code-marker.todo")
    );
}

#[test]
fn review_min_priority_preserves_in_diff_status_alignment() {
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

    let json = run_review_json(
        temp.path(),
        &[
            "review",
            ".",
            "--format",
            "json",
            "--scope",
            "full",
            "--min-priority",
            "p3",
        ],
    );
    let findings = json["findings"].as_array().unwrap();

    assert!(findings.iter().any(|finding| {
        finding["rule_id"] == "security.secret-candidate" && finding["in_diff"] == false
    }));
    assert!(
        findings.iter().any(|finding| {
            finding["rule_id"] == "code-marker.todo" && finding["in_diff"] == true
        })
    );
    assert!(json["review"]["in_diff_findings"].as_u64().unwrap() >= 1);
    assert!(json["review"]["out_of_diff_findings"].as_u64().unwrap() >= 1);
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

    let json = run_review_json(
        temp.path(),
        &[
            "review",
            "src/lib.rs",
            "--format",
            "json",
            "--profile",
            "strict",
        ],
    );

    assert!(json["review"]["in_diff_findings"].as_u64().unwrap() >= 1);
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
        &[
            "review",
            ".",
            "--base",
            base.trim(),
            "--format",
            "json",
            "--profile",
            "strict",
        ],
    );

    assert!(json["review"]["in_diff_findings"].as_u64().unwrap() >= 1);
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
        .args(["review", ".", "--scope", "full", "--fail-on", "new-high"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run review");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("CI gate: passed (new-high)"));
    assert!(stdout.contains("Out-of-diff findings: "));
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
fn review_marks_file_level_architecture_findings_in_diff_when_file_changed() {
    let temp = tempdir().expect("failed to create temp dir");
    init_repo(temp.path());
    fs::write(
        temp.path().join("repopilot.toml"),
        "[architecture]\nmax_fan_out = 1\ninstability_hub_min_fan_in = 999\n",
    )
    .expect("failed to write config");
    write_covered_source(temp.path(), "a", "use crate::b;\npub fn a() { b::b(); }\n");
    write_covered_source(temp.path(), "b", "pub fn b() {}\n");
    write_covered_source(temp.path(), "c", "pub fn c() {}\n");
    commit_all(temp.path(), "initial");

    fs::write(
        temp.path().join("src/a.rs"),
        "use crate::b;\nuse crate::c;\npub fn a() { b::b(); c::c(); }\n",
    )
    .expect("failed to modify source file");

    let json = run_review_json(temp.path(), &["review", ".", "--format", "json"]);

    assert_eq!(json["review"]["in_diff_findings"], 1);
    assert!(json["findings"].as_array().unwrap().iter().any(|finding| {
        finding["rule_id"] == "architecture.excessive-fan-out" && finding["in_diff"] == true
    }));
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

#[test]
fn review_writes_secondary_sarif_with_taint_signal() {
    let temp = tempdir().expect("failed to create temp dir");
    init_repo(temp.path());
    fs::create_dir_all(temp.path().join("src")).expect("create src");
    fs::write(
        temp.path().join("src/handler.ts"),
        "export function findUser() { return null; }\n",
    )
    .expect("write source");
    commit_all(temp.path(), "initial");
    fs::write(
        temp.path().join("src/handler.ts"),
        "export function findUser(req: Request) {\n  const id = req.query.id;\n  return db.query(\"SELECT * FROM users WHERE id = \" + id);\n}\n",
    )
    .expect("modify source");

    let sarif = temp.path().join("review.sarif");
    let output = repopilot()
        .args([
            "review",
            ".",
            "--format",
            "json",
            "--output",
            "review.json",
            "--sarif-output",
            sarif.to_str().unwrap(),
        ])
        .current_dir(temp.path())
        .output()
        .expect("run review");
    assert!(output.status.success());
    let value: Value =
        serde_json::from_slice(&fs::read(sarif).expect("read sarif")).expect("parse sarif");
    assert!(
        value["runs"][0]["results"]
            .as_array()
            .unwrap()
            .iter()
            .any(|result| result["ruleId"] == "taint.sql")
    );
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
