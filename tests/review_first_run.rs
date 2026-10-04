//! A first `repopilot review` outside Git or in a repository without commits
//! fails with a message that says what to run instead, not raw Git output.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::tempdir;

fn review(dir: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
        .args(["review", "."])
        .current_dir(dir)
        .env("GIT_CEILING_DIRECTORIES", dir.parent().expect("parent"))
        .output()
        .expect("repopilot runs")
}

#[test]
fn review_outside_git_points_to_scan() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("app.py"), "print(1)\n").expect("file");

    let output = review(dir.path());
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(
        stderr.contains("is not inside a Git repository"),
        "{stderr}"
    );
    assert!(stderr.contains("repopilot scan"), "{stderr}");
    assert!(!stderr.contains("fatal:"), "{stderr}");
}

#[test]
fn review_without_commits_says_so() {
    let dir = tempdir().expect("tempdir");
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .status()
        .expect("git runs");
    assert!(status.success());
    fs::write(dir.path().join("app.js"), "export const x = 1;\n").expect("file");

    let output = review(dir.path());
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("no commits yet"), "{stderr}");
    assert!(!stderr.contains("bad revision"), "{stderr}");
}
