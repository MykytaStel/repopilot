//! Shared setup for the agent hook precision tests.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use tempfile::{TempDir, tempdir};

pub const CLAUDE_SNAPSHOT: &str = "integrations/claude-code/repopilot/scripts/snapshot.sh";
pub const CLAUDE_GUARD: &str = "integrations/claude-code/repopilot/scripts/guard.sh";
pub const CURSOR_SNAPSHOT: &str = "integrations/cursor/hooks/repopilot-snapshot.sh";
pub const CURSOR_GUARD: &str = "integrations/cursor/hooks/repopilot-guard.sh";
pub const STARTUP: &str = r#"{"hook_event_name":"SessionStart","source":"startup"}"#;
pub const STOP: &str = r#"{"hook_event_name":"Stop","stop_hook_active":false}"#;
pub const CURSOR_STOP: &str = r#"{"status":"completed","loop_count":0}"#;

pub const TESTS: &str = r#"import { describe, expect, it } from "vitest";

describe("cart", () => {
  it("sums line items", () => {
    expect(1 + 2).toBe(3);
  });

  it("applies the discount", () => {
    expect(10 * 0.9).toBe(9);
  });
});
"#;
pub const WORKFLOW: &str = "name: ci\non: push\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - run: npm test\n";

pub fn run(root: &Path, script: &str, stdin: &str) -> Output {
    let bin_dir = Path::new(env!("CARGO_BIN_EXE_repopilot"))
        .parent()
        .expect("binary directory")
        .to_path_buf();
    let path = format!(
        "{}:{}",
        bin_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let mut child = Command::new("sh")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join(script))
        .current_dir(root)
        .env("PATH", path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("hook starts");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(stdin.as_bytes())
        .expect("stdin written");
    child.wait_with_output().expect("hook finishes")
}

pub fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("git runs");
    assert!(output.status.success(), "git {args:?}");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// A committed repository with a test file and one CI workflow.
pub fn repo(extra: &[(&str, &str)]) -> TempDir {
    let temp = tempdir().expect("temp repo");
    let root = temp.path();
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "repopilot@example.invalid"]);
    git(root, &["config", "user.name", "RepoPilot Test"]);
    let files = [
        ("src/cart.test.ts", TESTS),
        ("package.json", r#"{"dependencies":{"left-pad":"1.0.0"}}"#),
        (".github/workflows/ci.yml", WORKFLOW),
    ];
    for (path, content) in files.iter().chain(extra) {
        write(root, path, content);
    }
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "before"]);
    temp
}

pub fn write(root: &Path, path: &str, content: &str) {
    let path: PathBuf = root.join(path);
    fs::create_dir_all(path.parent().expect("parent")).expect("parent dir");
    fs::write(path, content).expect("write file");
}

pub fn start(root: &Path) {
    assert!(run(root, CLAUDE_SNAPSHOT, STARTUP).status.success());
}

/// Stop the Claude Code session; returns the exit code and the feedback.
pub fn stop(root: &Path) -> (Option<i32>, String) {
    let output = run(root, CLAUDE_GUARD, STOP);
    let feedback = String::from_utf8_lossy(&output.stderr).into_owned();
    (output.status.code(), feedback)
}

pub fn tests_with_skips(names: &[&str]) -> String {
    names.iter().fold(TESTS.to_string(), |text, name| {
        text.replace(&format!("it(\"{name}\""), &format!("it.skip(\"{name}\""))
    })
}
