//! The default `review` first screen: one verdict, honest coverage, recorded
//! revisions, and one next action (v0.23 ledger RP23-015 to RP23-019).

use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

#[test]
fn default_console_leads_with_one_verdict_and_hides_compatibility_records() {
    let temp = tempdir().expect("temp repo");
    let (base, head) = code_and_test_change(temp.path());

    let console = review(temp.path(), &["--base", &base, "--head", &head]);

    assert_eq!(console.matches("Decision: ").count(), 1, "{console}");
    assert!(console.contains("(Change Proof: "), "{console}");
    assert_eq!(console.matches("Next action: ").count(), 1, "{console}");
    for duplicate in [
        "Decision why:",
        "Decision limitation:",
        "Decision gates:",
        "Legacy merge readiness",
        "Evidence provenance",
        "Git root:",
    ] {
        assert!(!console.contains(duplicate), "{duplicate} in:\n{console}");
    }

    assert!(!console.contains('\u{1b}'), "ANSI escape with NO_COLOR");

    let full = review(
        temp.path(),
        &["--base", &base, "--head", &head, "--detail", "full"],
    );
    assert!(full.contains("Legacy merge readiness"));
    assert!(full.contains("Evidence provenance"));
}

#[test]
fn changed_tests_are_policy_skipped_not_coverage_exclusions() {
    let temp = tempdir().expect("temp repo");
    let (base, head) = code_and_test_change(temp.path());

    let json = review_json(temp.path(), &["--base", &base, "--head", &head]);
    let coverage = &json["change_proof"]["coverage"];

    assert_eq!(coverage["requested_files"], 2, "{coverage}");
    assert_eq!(coverage["excluded_files"], 0, "{coverage}");
    assert_eq!(coverage["unsupported_files"], 0, "{coverage}");
    assert_eq!(coverage["policy_skipped_files"], 1, "{coverage}");
    let reasons = json["change_proof"]["reasons"].as_array().expect("reasons");
    assert!(
        !reasons
            .iter()
            .any(|reason| reason["code"] == "scope-coverage-incomplete"),
        "{reasons:?}"
    );
}

#[test]
fn explicit_refs_are_recorded_as_resolved_commits() {
    let temp = tempdir().expect("temp repo");
    let (base, head) = code_and_test_change(temp.path());

    let json = review_json(temp.path(), &["--base", &base, "--head", &head]);
    let provenance = &json["evidence"]["provenance"];

    assert_eq!(provenance["base_commit"], base.as_str(), "{provenance}");
    assert_eq!(provenance["head_commit"], head.as_str(), "{provenance}");
    let unavailable = provenance["unavailable_inputs"]
        .as_array()
        .expect("unavailable inputs");
    for input in ["base revision", "head revision", "current revision"] {
        assert!(
            !unavailable.iter().any(|value| value == input),
            "{input} reported unavailable: {provenance}"
        );
    }
}

#[test]
fn configured_passing_check_reaches_verified() {
    let temp = tempdir().expect("temp repo");
    let root = temp.path();
    init(root);
    write(
        root,
        "repopilot.toml",
        "[[verification.checks]]\nid = \"unit\"\nrole = \"test\"\nprogram = \"git\"\nargs = [\"--version\"]\npaths = [\"src/**\"]\n",
    );
    write(root, "src/lib.rs", "pub fn value() -> u32 {\n    1\n}\n");
    git(root, &["add", "."]);
    git(root, &["commit", "-q", "-m", "base"]);
    write(
        root,
        "src/lib.rs",
        "/// The configured value.\npub fn value() -> u32 {\n    1\n}\n",
    );

    let unverified = review_json(root, &[]);
    assert_eq!(unverified["change_proof"]["verdict"], "REVIEW");
    let next_action = unverified["decision"]["next_action"]
        .as_str()
        .expect("next action");
    assert!(
        next_action.contains("repopilot init --suggestions-output"),
        "{next_action}"
    );

    let verified = review_json(root, &["--verify", "unit"]);
    assert_eq!(
        verified["change_proof"]["verdict"], "VERIFIED",
        "{}",
        verified["change_proof"]
    );
}

#[test]
fn html_report_keeps_basic_accessibility_landmarks() {
    let temp = tempdir().expect("temp repo");
    let (base, head) = code_and_test_change(temp.path());

    let html = review(
        temp.path(),
        &["--base", &base, "--head", &head, "--format", "html"],
    );

    assert!(html.contains("<html lang=\"en\">"));
    assert_eq!(html.matches("<h1>").count(), 1, "exactly one page heading");
    assert!(html.contains("aria-labelledby=\"proof-heading\""));
    assert!(
        html.contains("id=\"proof-heading\""),
        "labelled-by target exists"
    );
    assert!(!html.contains("<script src="), "no remote scripts");
}

#[test]
fn reviewing_a_nested_repository_from_its_parent_sees_the_change() {
    let outer = tempdir().expect("outer dir");
    let inner = outer.path().join("checkout");
    fs::create_dir_all(&inner).expect("inner dir");
    init(&inner);
    write(&inner, "src/lib.rs", "pub fn value() -> u32 {\n    1\n}\n");
    git(&inner, &["add", "."]);
    git(&inner, &["commit", "-q", "-m", "base"]);
    write(&inner, "src/lib.rs", "pub fn value() -> u32 {\n    2\n}\n");

    let output = repopilot()
        .args(["review", "checkout", "--format", "json", "--no-progress"])
        .current_dir(outer.path())
        .output()
        .expect("run review");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("review JSON");

    assert_eq!(
        json["changed_files"].as_array().map(Vec::len),
        Some(1),
        "{}",
        json["changed_files"]
    );
    assert_ne!(json["change_proof"]["verdict"], "NOT_ASSESSED");
}

fn init(root: &Path) {
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "test@repopilot.local"]);
    git(root, &["config", "user.name", "RepoPilot Test"]);
}

fn code_and_test_change(root: &Path) -> (String, String) {
    init(root);
    write(root, "src/lib.rs", "pub fn value() -> u32 {\n    1\n}\n");
    write(
        root,
        "tests/value_tests.rs",
        "#[test]\nfn value_is_one() {\n    assert_eq!(1, 1);\n}\n",
    );
    git(root, &["add", "."]);
    git(root, &["commit", "-q", "-m", "base"]);
    let base = rev_parse(root);
    write(root, "src/lib.rs", "pub fn value() -> u32 {\n    2\n}\n");
    write(
        root,
        "tests/value_tests.rs",
        "#[test]\nfn value_is_two() {\n    assert_eq!(2, 2);\n}\n",
    );
    git(root, &["commit", "-q", "-am", "change"]);
    (base, rev_parse(root))
}

fn review(root: &Path, extra: &[&str]) -> String {
    let output = repopilot()
        .args(["review", ".", "--no-progress"])
        .args(extra)
        .env("NO_COLOR", "1")
        .current_dir(root)
        .output()
        .expect("run review");
    assert!(
        output.status.success(),
        "review failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("UTF-8 review output")
}

fn review_json(root: &Path, extra: &[&str]) -> Value {
    let mut args = vec!["--format", "json"];
    args.extend_from_slice(extra);
    serde_json::from_str(&review(root, &args)).expect("review JSON")
}

fn rev_parse(root: &Path) -> String {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()
        .expect("run git rev-parse");
    String::from_utf8(output.stdout)
        .expect("UTF-8 sha")
        .trim()
        .to_string()
}

fn write(root: &Path, path: &str, content: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
    fs::write(path, content).expect("write fixture");
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
