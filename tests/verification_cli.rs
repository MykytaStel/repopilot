use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

#[cfg(unix)]
#[test]
fn selected_check_runs_and_is_reported_once() {
    let temp = tempdir().expect("temp dir");
    init_repo(temp.path());
    fs::create_dir_all(temp.path().join("src")).expect("src");
    fs::write(temp.path().join("src/lib.rs"), "pub fn before() {}\n").expect("source");
    fs::write(
        temp.path().join("repopilot.toml"),
        r#"
        [[verification.checks]]
        id = "unit"
        role = "test"
        program = "sh"
        args = ["-c", "printf verified"]
        "#,
    )
    .expect("config");
    commit_all(temp.path(), "initial");
    fs::write(temp.path().join("src/lib.rs"), "pub fn after() {}\n").expect("change");

    let output = repopilot()
        .args([
            "review", ".", "--format", "json", "--verify", "unit", "--verify", "unit",
        ])
        .current_dir(temp.path())
        .output()
        .expect("review");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("JSON report");
    let outcomes = json["merge_readiness"]["verification"]
        .as_array()
        .expect("outcomes");
    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0]["check_id"], "unit");
    assert_eq!(outcomes[0]["status"], "passed");
    assert_eq!(outcomes[0]["working_directory"], ".");
    assert_eq!(outcomes[0]["stdout_excerpt"], "verified");
}

#[test]
fn unknown_selected_check_exits_with_usage_code() {
    let temp = tempdir().expect("temp dir");
    init_repo(temp.path());
    fs::write(temp.path().join("README.md"), "before\n").expect("source");
    commit_all(temp.path(), "initial");
    fs::write(temp.path().join("README.md"), "after\n").expect("change");

    let output = repopilot()
        .args(["review", ".", "--verify", "missing"])
        .current_dir(temp.path())
        .output()
        .expect("review");

    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unknown verification check id `missing`")
    );
}

#[cfg(unix)]
#[test]
fn console_renders_verification_evidence() {
    let temp = tempdir().expect("temp dir");
    init_repo(temp.path());
    fs::write(temp.path().join("README.md"), "before\n").expect("source");
    fs::write(
        temp.path().join("repopilot.toml"),
        "[[verification.checks]]\nid = \"lint\"\nrole = \"lint\"\nprogram = \"sh\"\nargs = [\"-c\", \"printf clean\"]\n",
    )
    .expect("config");
    commit_all(temp.path(), "initial");
    fs::write(temp.path().join("README.md"), "after\n").expect("change");

    let output = repopilot()
        .args(["review", ".", "--verify", "lint"])
        .current_dir(temp.path())
        .output()
        .expect("review");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("Verification:\n"));
    assert!(stdout.contains("lint: PASSED"));
    assert!(stdout.contains("clean"));
}

#[cfg(unix)]
#[test]
fn ref_range_verification_rejects_a_non_checkout_head() {
    let temp = tempdir().expect("temp dir");
    init_repo(temp.path());
    fs::write(temp.path().join("README.md"), "one\n").expect("source");
    fs::write(
        temp.path().join("repopilot.toml"),
        "[[verification.checks]]\nid = \"unit\"\nrole = \"test\"\nprogram = \"sh\"\nargs = [\"-c\", \"printf should-not-run > marker\"]\n",
    )
    .expect("config");
    commit_all(temp.path(), "first");
    fs::write(temp.path().join("README.md"), "two\n").expect("change");
    commit_all(temp.path(), "second");

    let output = repopilot()
        .args([
            "review", ".", "--base", "HEAD~1", "--head", "HEAD~1", "--verify", "unit",
        ])
        .current_dir(temp.path())
        .output()
        .expect("review");

    assert_eq!(output.status.code(), Some(2));
    assert!(!temp.path().join("marker").exists());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("does not match the current checkout")
    );
}

#[cfg(unix)]
#[test]
fn ref_range_allows_changes_covered_by_configured_ignores() {
    let temp = tempdir().expect("temp dir");
    init_repo(temp.path());
    fs::write(temp.path().join("README.md"), "one\n").expect("source");
    fs::write(
        temp.path().join("repopilot.toml"),
        "[scan]\nignore = [\"generated\"]\n[[verification.checks]]\nid = \"unit\"\nrole = \"test\"\nprogram = \"sh\"\nargs = [\"-c\", \"printf ok\"]\n",
    )
    .expect("config");
    commit_all(temp.path(), "first");
    fs::write(temp.path().join("README.md"), "two\n").expect("change");
    commit_all(temp.path(), "second");
    fs::create_dir_all(temp.path().join("generated")).expect("generated dir");
    fs::write(temp.path().join("generated/output.txt"), "ignored\n").expect("ignored file");

    let output = repopilot()
        .args([
            "review", ".", "--base", "HEAD~1", "--head", "HEAD", "--verify", "unit",
        ])
        .current_dir(temp.path())
        .output()
        .expect("review");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(unix)]
#[test]
fn configured_check_does_not_run_without_explicit_selection() {
    let temp = tempdir().expect("temp dir");
    init_repo(temp.path());
    fs::write(temp.path().join("README.md"), "before\n").expect("source");
    fs::write(
        temp.path().join("repopilot.toml"),
        "[[verification.checks]]\nid = \"unit\"\nrole = \"test\"\nprogram = \"sh\"\nargs = [\"-c\", \"printf spawned > marker\"]\n",
    )
    .expect("config");
    commit_all(temp.path(), "initial");
    fs::write(temp.path().join("README.md"), "after\n").expect("change");

    let output = repopilot()
        .args(["review", ".", "--format", "json"])
        .current_dir(temp.path())
        .output()
        .expect("review");
    let json: Value = serde_json::from_slice(&output.stdout).expect("JSON report");

    assert!(output.status.success());
    assert!(!temp.path().join("marker").exists());
    assert!(json["merge_readiness"].get("verification").is_none());
}

#[cfg(unix)]
#[test]
fn failed_check_writes_report_before_exit_one() {
    let temp = tempdir().expect("temp dir");
    init_repo(temp.path());
    fs::write(temp.path().join("README.md"), "before\n").expect("source");
    fs::write(
        temp.path().join("repopilot.toml"),
        "[[verification.checks]]\nid = \"unit\"\nrole = \"test\"\nprogram = \"sh\"\nargs = [\"-c\", \"printf failed >&2; exit 7\"]\n",
    )
    .expect("config");
    commit_all(temp.path(), "initial");
    fs::write(temp.path().join("README.md"), "after\n").expect("change");
    let report = temp.path().join("review.json");

    let output = repopilot()
        .args([
            "review",
            ".",
            "--format",
            "json",
            "--output",
            report.to_str().unwrap(),
            "--verify",
            "unit",
        ])
        .current_dir(temp.path())
        .output()
        .expect("review");
    let json: Value =
        serde_json::from_slice(&fs::read(report).expect("written report")).expect("JSON report");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        json["merge_readiness"]["verification"][0]["status"],
        "failed"
    );
    assert_eq!(json["merge_readiness"]["verification"][0]["exit_code"], 7);
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

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("git");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
