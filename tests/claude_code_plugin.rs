//! The Claude Code plugin's hook scripts, run as Claude Code runs them: JSON on
//! stdin, `repopilot` on `PATH`, exit 2 to block a stop.
#![cfg(unix)]

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use tempfile::tempdir;

const BEFORE: &str = r#"import { describe, expect, it } from "vitest";
import { total } from "./cart";

describe("cart", () => {
  it("sums line items", () => {
    expect(total([2, 3])).toBe(5);
    expect(total([])).toBe(0);
  });

  it("applies the discount", () => {
    expect(total([10], 0.1)).toBe(9);
  });
});
"#;

fn plugin_script(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("integrations/claude-code/repopilot/scripts")
        .join(name)
}

fn hook(root: &Path, script: &str, stdin: &str) -> Output {
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
        .arg(plugin_script(script))
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

fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(root)
        .status()
        .expect("git runs");
    assert!(status.success(), "git {args:?}");
}

fn repo() -> tempfile::TempDir {
    let temp = tempdir().expect("temp repo");
    let root = temp.path();
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "repopilot@example.invalid"]);
    git(root, &["config", "user.name", "RepoPilot Test"]);
    fs::create_dir_all(root.join("src")).expect("src");
    fs::write(root.join("src/cart.test.ts"), BEFORE).expect("test file");
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "before"]);
    temp
}

#[test]
fn stop_hook_blocks_a_session_that_weakened_tests_and_clears_after_restore() {
    let temp = repo();
    let root = temp.path();
    let start = hook(
        root,
        "snapshot.sh",
        r#"{"hook_event_name":"SessionStart","source":"startup"}"#,
    );
    assert!(start.status.success());
    assert!(root.join(".repopilot/snapshot.json").is_file());

    // The agent skips one test and drops an assertion from the other.
    let weakened = BEFORE
        .replace(
            "it(\"applies the discount\"",
            "it.skip(\"applies the discount\"",
        )
        .replace("    expect(total([])).toBe(0);\n", "");
    fs::write(root.join("src/cart.test.ts"), &weakened).expect("weaken");

    let stop = hook(
        root,
        "guard.sh",
        r#"{"hook_event_name":"Stop","stop_hook_active":false}"#,
    );
    let feedback = String::from_utf8_lossy(&stop.stderr);
    assert_eq!(stop.status.code(), Some(2), "{feedback}");
    assert!(
        feedback.contains("test skipped — src/cart.test.ts"),
        "{feedback}"
    );
    assert!(
        feedback.contains("assertions removed — src/cart.test.ts"),
        "{feedback}"
    );

    let again = hook(
        root,
        "guard.sh",
        r#"{"hook_event_name":"Stop","stop_hook_active":true}"#,
    );
    assert_eq!(
        again.status.code(),
        Some(0),
        "the hook re-prompts only once"
    );

    fs::write(root.join("src/cart.test.ts"), BEFORE).expect("restore");
    let restored = hook(
        root,
        "guard.sh",
        r#"{"hook_event_name":"Stop","stop_hook_active":false}"#,
    );
    assert_eq!(
        restored.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&restored.stderr)
    );
}

#[test]
fn hooks_stay_out_of_the_way_outside_git_and_on_resume() {
    let temp = tempdir().expect("temp dir");
    let outside = hook(temp.path(), "guard.sh", r#"{"stop_hook_active":false}"#);
    assert_eq!(outside.status.code(), Some(0));

    let repo = repo();
    let resumed = hook(repo.path(), "snapshot.sh", r#"{"source":"resume"}"#);
    assert!(resumed.status.success());
    assert!(!repo.path().join(".repopilot/snapshot.json").exists());
}

#[cfg(unix)]
#[test]
fn session_start_does_not_write_through_a_repopilot_symlink() {
    let repo = repo();
    let outside = tempdir().expect("outside dir");
    let sentinel = outside.path().join("snapshot.json");
    fs::write(&sentinel, "keep outside data").expect("sentinel");
    std::os::unix::fs::symlink(outside.path(), repo.path().join(".repopilot"))
        .expect("symlink repopilot directory");

    let output = hook(
        repo.path(),
        "snapshot.sh",
        r#"{"hook_event_name":"SessionStart","source":"startup"}"#,
    );

    assert!(output.status.success());
    assert_eq!(
        fs::read_to_string(sentinel).expect("sentinel survives"),
        "keep outside data"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("symlink"));
}

#[cfg(unix)]
#[test]
fn session_start_preserves_and_rejects_a_symlinked_snapshot_file() {
    let repo = repo();
    let outside = tempdir().expect("outside dir");
    let sentinel = outside.path().join("snapshot.json");
    fs::write(&sentinel, "keep outside data").expect("sentinel");
    fs::create_dir(repo.path().join(".repopilot")).expect("state directory");
    let snapshot = repo.path().join(".repopilot/snapshot.json");
    std::os::unix::fs::symlink(&sentinel, &snapshot).expect("symlink snapshot file");

    let output = hook(
        repo.path(),
        "snapshot.sh",
        r#"{"hook_event_name":"SessionStart","source":"startup"}"#,
    );

    assert!(output.status.success());
    assert!(
        fs::symlink_metadata(snapshot)
            .expect("snapshot symlink remains")
            .file_type()
            .is_symlink()
    );
    assert_eq!(
        fs::read_to_string(sentinel).expect("sentinel survives"),
        "keep outside data"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("snapshot.json must not be a symlink")
    );
}

#[test]
fn silencing_repopilot_from_the_session_still_blocks_the_stop() {
    let temp = repo();
    let root = temp.path();
    assert!(
        hook(root, "snapshot.sh", r#"{"source":"startup"}"#)
            .status
            .success()
    );

    // The agent skips a test and then tries to acknowledge its own skip.
    let weakened = BEFORE.replace(
        "it(\"applies the discount\"",
        "it.skip(\"applies the discount\"",
    );
    fs::write(root.join("src/cart.test.ts"), weakened).expect("weaken");
    fs::create_dir_all(root.join(".repopilot")).expect("overlay dir");
    let mut overlay = fs::read_to_string(root.join(".repopilot/overlay.toml")).unwrap_or_default();
    overlay.push_str(
        "[[overlay]]\nkind = \"integrity.test-skipped\"\npath = \"src/**\"\nreason = \"flaky\"\n",
    );
    fs::write(root.join(".repopilot/overlay.toml"), overlay).expect("overlay");

    let stop = hook(root, "guard.sh", r#"{"stop_hook_active":false}"#);
    let feedback = String::from_utf8_lossy(&stop.stderr);
    assert_eq!(stop.status.code(), Some(2), "{feedback}");
    assert!(
        feedback.contains("RepoPilot suppression added"),
        "{feedback}"
    );
    assert!(feedback.contains("integrity.test-skipped"), "{feedback}");
}
