//! Integrity false positives found in agent sessions, each with a recall
//! guard: a rewritten CI test command is not a removed check, and skipping
//! every test in a file is a skip, not an emptied test file.

use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

const WORKFLOW: &str = "name: ci\non: push\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - run: npm test\n";
const TESTS: &str = "import { expect, it } from \"vitest\";\n\nit(\"adds\", () => {\n  expect(1 + 2).toBe(3);\n});\n\nit(\"subtracts\", () => {\n  expect(3 - 2).toBe(1);\n});\n";

fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(root)
        .status()
        .expect("git runs");
    assert!(status.success(), "git {args:?}");
}

fn write(root: &Path, path: &str, content: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().expect("parent")).expect("parent dir");
    fs::write(path, content).expect("write file");
}

fn repo(root: &Path) {
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "repopilot@example.invalid"]);
    git(root, &["config", "user.name", "RepoPilot Test"]);
    write(root, ".github/workflows/ci.yml", WORKFLOW);
    write(root, "src/math.test.ts", TESTS);
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "before"]);
}

/// Kinds of the unsuppressed review signals for the working-tree change.
fn signal_kinds(root: &Path) -> Vec<String> {
    let output = Command::new(env!("CARGO_BIN_EXE_repopilot"))
        .args(["review", ".", "--format", "json"])
        .current_dir(root)
        .output()
        .expect("review runs");
    assert!(output.status.success(), "{output:?}");
    let report: Value = serde_json::from_slice(&output.stdout).expect("review JSON");
    ["definitely", "maybe", "noise"]
        .iter()
        .flat_map(|tier| {
            report["tiered_signals"][tier]
                .as_array()
                .cloned()
                .unwrap_or_default()
        })
        .filter(|signal| signal["suppressed"] != true)
        .filter_map(|signal| signal["kind"].as_str().map(str::to_string))
        .collect()
}

#[test]
fn a_rewritten_ci_test_command_is_not_a_removed_check() {
    let temp = tempdir().expect("temp repo");
    let root = temp.path();
    repo(root);
    write(
        root,
        ".github/workflows/ci.yml",
        &WORKFLOW.replace("npm test", "npm run test:ci"),
    );
    let kinds = signal_kinds(root);
    assert!(
        !kinds.contains(&"integrity.gate-relaxed".to_string()),
        "{kinds:?}"
    );

    write(
        root,
        ".github/workflows/ci.yml",
        &WORKFLOW.replace("npm test", "npm run lint"),
    );
    let kinds = signal_kinds(root);
    assert!(
        kinds.contains(&"integrity.gate-relaxed".to_string()),
        "a test step replaced by lint still weakens the gate: {kinds:?}"
    );
}

#[test]
fn skipping_every_test_is_a_skip_not_an_emptied_test_file() {
    let temp = tempdir().expect("temp repo");
    let root = temp.path();
    repo(root);
    write(
        root,
        "src/math.test.ts",
        &TESTS.replace("it(\"", "it.skip(\""),
    );
    let kinds = signal_kinds(root);
    assert!(
        kinds.contains(&"integrity.test-skipped".to_string()),
        "{kinds:?}"
    );
    assert!(
        !kinds.contains(&"behavioral.test-deleted-or-emptied".to_string()),
        "{kinds:?}"
    );

    write(
        root,
        "src/math.test.ts",
        "import { expect, it } from \"vitest\";\n",
    );
    let kinds = signal_kinds(root);
    assert!(
        kinds.contains(&"behavioral.test-deleted-or-emptied".to_string()),
        "removing every test is still an emptied test file: {kinds:?}"
    );
}
