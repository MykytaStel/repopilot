//! End-to-end coverage for the `review` security-boundary change signals
//! (`src/review/signals.rs`). Unit-level true/false-positive cases live next to
//! the detector; these tests prove the signals flow through the real `review`
//! command and the JSON report (the agent-facing `repopilot_review_change` shape).

use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

#[test]
fn review_flags_boundary_changes_in_json() {
    let temp = tempdir().expect("failed to create temp dir");
    init_repo(temp.path());
    write_covered_source(temp.path(), "lib", "pub fn live() {}\n");
    commit_all(temp.path(), "initial");

    // Three untracked boundary files across distinct categories.
    write(
        temp.path(),
        "src/auth/session.ts",
        "export const session = {};\n",
    );
    write(
        temp.path(),
        ".github/workflows/deploy.yml",
        "name: deploy\non: push\njobs: {}\n",
    );
    write(temp.path(), "package.json", "{\n  \"name\": \"demo\"\n}\n");

    let json = run_review_json(temp.path(), &["review", ".", "--format", "json"]);

    let signals = json["boundary_signals"]
        .as_array()
        .expect("boundary_signals array");
    assert_eq!(json["review"]["boundary_signals"], signals.len() as u64);

    let by_category = |category: &str, path: &str| {
        signals
            .iter()
            .any(|signal| signal["category"] == category && signal["path"] == path)
    };

    assert!(by_category("access-control", "src/auth/session.ts"));
    assert!(by_category(
        "deploy-surface",
        ".github/workflows/deploy.yml"
    ));
    assert!(by_category("supply-chain", "package.json"));
}

#[test]
fn review_omits_boundary_signals_for_ordinary_changes() {
    let temp = tempdir().expect("failed to create temp dir");
    init_repo(temp.path());
    write_covered_source(temp.path(), "lib", "pub fn live() {}\n");
    commit_all(temp.path(), "initial");

    fs::write(
        temp.path().join("src/lib.rs"),
        "pub fn live() {}\n// TODO: ordinary change\n",
    )
    .expect("failed to modify source file");

    let json = run_review_json(temp.path(), &["review", ".", "--format", "json"]);

    assert_eq!(json["review"]["boundary_signals"], 0);
    assert!(json["boundary_signals"].as_array().unwrap().is_empty());
}

#[test]
fn review_console_shows_boundary_section() {
    let temp = tempdir().expect("failed to create temp dir");
    init_repo(temp.path());
    write_covered_source(temp.path(), "lib", "pub fn live() {}\n");
    commit_all(temp.path(), "initial");

    write(
        temp.path(),
        "src/middleware/auth.ts",
        "export const auth = () => {};\n",
    );

    let output = repopilot()
        .args(["review", "."])
        .current_dir(temp.path())
        .output()
        .expect("failed to run review");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Security boundary changed"));
    assert!(stdout.contains("access control"));
    assert!(stdout.contains("src/middleware/auth.ts"));
}

#[test]
fn review_respects_disabled_boundary_config() {
    let temp = tempdir().expect("failed to create temp dir");
    init_repo(temp.path());
    write_covered_source(temp.path(), "lib", "pub fn live() {}\n");
    commit_all(temp.path(), "initial");

    fs::write(
        temp.path().join("repopilot.toml"),
        "[security_boundary]\nenabled = false\n",
    )
    .expect("failed to write config");
    write(
        temp.path(),
        "src/middleware/auth.ts",
        "export const auth = () => {};\n",
    );

    let json = run_review_json(temp.path(), &["review", ".", "--format", "json"]);

    assert_eq!(json["review"]["boundary_signals"], 0);
}

#[test]
fn review_reports_blast_radius_for_boundary_file() {
    let temp = tempdir().expect("failed to create temp dir");
    init_repo(temp.path());
    write_covered_source(temp.path(), "lib", "pub fn live() {}\n");
    write(
        temp.path(),
        "src/auth/session.ts",
        "export const session = 1;\n",
    );
    write(
        temp.path(),
        "src/app.ts",
        "import { session } from \"./auth/session\";\nexport const app = session;\n",
    );
    commit_all(temp.path(), "initial");

    // Modify the boundary file so it lands in the diff; app.ts still imports it.
    fs::write(
        temp.path().join("src/auth/session.ts"),
        "export const session = 2;\n",
    )
    .expect("failed to modify boundary file");

    let json = run_review_json(temp.path(), &["review", ".", "--format", "json"]);

    let session = json["boundary_signals"]
        .as_array()
        .unwrap()
        .iter()
        .find(|signal| signal["path"] == "src/auth/session.ts")
        .expect("session.ts boundary signal");
    assert_eq!(session["category"], "access-control");
    assert_eq!(session["blast_radius"], 1);
}

#[test]
fn review_flags_code_boundary_without_a_test_change() {
    let temp = tempdir().expect("failed to create temp dir");
    init_repo(temp.path());
    write_covered_source(temp.path(), "lib", "pub fn live() {}\n");
    commit_all(temp.path(), "initial");

    write(
        temp.path(),
        "src/middleware/auth.ts",
        "export const auth = () => {};\n",
    );

    let json = run_review_json(temp.path(), &["review", ".", "--format", "json"]);
    assert_eq!(json["review"]["boundary_missing_test"], true);
}

#[test]
fn review_silent_when_boundary_change_includes_a_test() {
    let temp = tempdir().expect("failed to create temp dir");
    init_repo(temp.path());
    write_covered_source(temp.path(), "lib", "pub fn live() {}\n");
    commit_all(temp.path(), "initial");

    write(
        temp.path(),
        "src/middleware/auth.ts",
        "export const auth = () => {};\n",
    );
    write(
        temp.path(),
        "tests/auth.test.ts",
        "test('auth', () => {});\n",
    );

    let json = run_review_json(temp.path(), &["review", ".", "--format", "json"]);
    assert_eq!(json["review"]["boundary_missing_test"], false);
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

fn write(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("failed to create parent dir");
    }
    fs::write(path, content).expect("failed to write file");
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
