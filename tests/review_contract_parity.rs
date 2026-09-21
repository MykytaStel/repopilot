//! Pipeline parity for ChangeProof contract deltas.
//!
//! A semantic contract is useful only if its evidence survives the supported
//! review entry points and cache states. This fixture compares the canonical
//! contract array across the working tree, snapshot, cold-cache, warm-cache,
//! and explicit base/head review paths.

use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

#[test]
fn security_contracts_are_stable_across_review_entry_points_and_cache_states() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    init_repo(root);
    write(
        root,
        "src/auth/policy.ts",
        "export const policy = () => true;\n",
    );
    write(
        root,
        "src/routes/users.ts",
        "import { policy } from \"../auth/policy\";\nexport const users = () => policy();\n",
    );
    write(root, "tests/auth.test.ts", "test('auth', () => {});\n");
    git(root, &["add", "."]);
    git(root, &["commit", "-m", "before"]);
    let base = git_output(root, &["rev-parse", "HEAD"]);
    run(root, &["snapshot"]);

    write(
        root,
        "src/auth/policy.ts",
        "export const policy = () => false;\n",
    );
    write(
        root,
        "tests/auth.test.ts",
        "test('auth', () => expect(true).toBe(true));\n",
    );

    let working = review_json(root, &["review", "."]);
    let snapshot = review_json(root, &["review", ".", "--since-snapshot"]);

    let _ = fs::remove_dir_all(root.join(".repopilot/cache"));
    let cold = review_json(root, &["review", "."]);
    let warm = review_json(root, &["review", "."]);

    git(root, &["add", "src/auth/policy.ts", "tests/auth.test.ts"]);
    git(root, &["commit", "-m", "after"]);
    let refs = review_json(root, &["review", ".", "--base", &base, "--head", "HEAD"]);

    let expected = contract_deltas(&working);
    for (label, report) in [
        ("snapshot", &snapshot),
        ("cold", &cold),
        ("warm", &warm),
        ("refs", &refs),
    ] {
        assert_eq!(
            contract_deltas(report),
            expected,
            "{label} review changed ChangeProof contract evidence"
        );
    }

    assert!(expected.iter().any(|delta| {
        delta["family"] == "security-boundary" && delta["change"] == "boundary-changed"
    }));
    assert!(expected.iter().any(|delta| {
        delta["family"] == "security-boundary"
            && delta["change"] == "entry-point-impacted"
            && delta["consumer_path"] == "src/routes/users.ts"
    }));
    assert!(expected.iter().any(|delta| {
        delta["family"] == "test-coverage"
            && delta["change"] == "test-changed"
            && delta["consumer_path"] == "tests/auth.test.ts"
    }));
}

#[test]
fn unrelated_changed_test_stays_explicitly_unlinked() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    init_repo(root);
    write(
        root,
        "src/auth/policy.ts",
        "export const policy = () => true;\n",
    );
    write(root, "tests/other.test.ts", "test('other', () => {});\n");
    git(root, &["add", "."]);
    git(root, &["commit", "-m", "before"]);

    write(
        root,
        "src/auth/policy.ts",
        "export const policy = () => false;\n",
    );
    write(
        root,
        "tests/other.test.ts",
        "test('other', () => expect(true).toBe(true));\n",
    );
    let json = review_json(root, &["review", "."]);
    let deltas = contract_deltas(&json);
    assert!(
        deltas.iter().any(|delta| {
            delta["family"] == "test-coverage" && delta["change"] == "test-missing"
        })
    );
    assert!(
        !deltas.iter().any(|delta| {
            delta["family"] == "test-coverage" && delta["change"] == "test-changed"
        })
    );
}

fn contract_deltas(json: &Value) -> Vec<Value> {
    json["change_proof"]["contract_deltas"]
        .as_array()
        .cloned()
        .expect("change_proof.contract_deltas array")
}

fn review_json(root: &Path, args: &[&str]) -> Value {
    let mut command = repopilot();
    command
        .args(args)
        .args(["--format", "json"])
        .current_dir(root);
    let output = command.output().expect("run review");
    assert!(
        output.status.success(),
        "review failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("review JSON")
}

fn init_repo(root: &Path) {
    git(root, &["init"]);
    git(root, &["config", "user.email", "repopilot@example.invalid"]);
    git(root, &["config", "user.name", "RepoPilot Test"]);
}

fn write(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
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

fn git_output(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("run git");
    assert!(output.status.success(), "git {args:?} failed");
    String::from_utf8(output.stdout)
        .expect("git output utf8")
        .trim()
        .to_string()
}

fn run(root: &Path, args: &[&str]) {
    let output = repopilot()
        .args(args)
        .current_dir(root)
        .output()
        .expect("run repopilot");
    assert!(
        output.status.success(),
        "command {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
