//! The Cursor hook recipe, run as Cursor runs it: JSON on stdin, `repopilot` on
//! `PATH`, and a `followup_message` on stdout when the session weakened checks.
#![cfg(unix)]

use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
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

fn recipe() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("integrations/cursor")
}

/// Runs a hook script and returns its stdout parsed as JSON.
fn hook(root: &Path, script: &str, stdin: &str) -> Value {
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
        .arg(recipe().join("hooks").join(script))
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
    let output = child.wait_with_output().expect("hook finishes");
    assert!(output.status.success(), "{output:?}");
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "hook output is not JSON ({error}): {}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
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
fn stop_hook_follows_up_on_weakened_tests_once_and_clears_after_restore() {
    let temp = repo();
    let root = temp.path();
    let start = hook(
        root,
        "repopilot-snapshot.sh",
        r#"{"session_id":"s1","is_background_agent":false,"composer_mode":"agent"}"#,
    );
    assert_eq!(start, serde_json::json!({}));
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
        "repopilot-guard.sh",
        r#"{"status":"completed","loop_count":0}"#,
    );
    let message = stop["followup_message"]
        .as_str()
        .expect("follow-up message");
    assert!(
        message.contains("test skipped — src/cart.test.ts"),
        "{message}"
    );
    assert!(
        message.contains("assertions removed — src/cart.test.ts"),
        "{message}"
    );
    assert!(message.contains("\"applies the discount\""), "{message}");

    for input in [
        r#"{"status":"completed","loop_count":1}"#,
        r#"{"status":"aborted","loop_count":0}"#,
    ] {
        assert_eq!(
            hook(root, "repopilot-guard.sh", input),
            serde_json::json!({}),
            "{input}"
        );
    }

    fs::write(root.join("src/cart.test.ts"), BEFORE).expect("restore");
    let restored = hook(
        root,
        "repopilot-guard.sh",
        r#"{"status":"completed","loop_count":0}"#,
    );
    assert_eq!(restored, serde_json::json!({}));
}

#[test]
fn hooks_stay_quiet_outside_git() {
    let temp = tempdir().expect("temp dir");
    let stop = hook(
        temp.path(),
        "repopilot-guard.sh",
        r#"{"status":"completed","loop_count":0}"#,
    );
    assert_eq!(stop, serde_json::json!({}));
    let start = hook(temp.path(), "repopilot-snapshot.sh", "{}");
    assert_eq!(start, serde_json::json!({}));
    assert!(!temp.path().join(".repopilot").exists());
}

#[test]
fn hooks_json_points_at_the_shipped_scripts() {
    let config: Value =
        serde_json::from_str(&fs::read_to_string(recipe().join("hooks.json")).expect("hooks.json"))
            .expect("hooks.json is JSON");
    assert_eq!(config["version"], 1);
    for event in ["sessionStart", "stop"] {
        let command = config["hooks"][event][0]["command"]
            .as_str()
            .expect("command");
        let shipped = command
            .strip_prefix(".cursor/")
            .expect("project hooks run from the project root");
        assert!(recipe().join(shipped).is_file(), "{command}");
    }
    assert_eq!(config["hooks"]["stop"][0]["loop_limit"], 1);
}
