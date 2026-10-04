//! A first `repopilot review` outside Git, in a repository without commits, or
//! with a base ref that does not exist fails with a message that says what to
//! run instead, not raw Git output.

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

fn committed_repo(dir: &Path) {
    for args in [
        &["init", "-q", "-b", "master"][..],
        &["config", "user.email", "repopilot@example.invalid"],
        &["config", "user.name", "RepoPilot Test"],
    ] {
        let status = Command::new("git")
            .args(args)
            .current_dir(dir)
            .status()
            .expect("git runs");
        assert!(status.success(), "git {args:?}");
    }
    fs::write(dir.join("app.js"), "export const x = 1;\n").expect("file");
    for args in [&["add", "."][..], &["commit", "-qm", "init"]] {
        let status = Command::new("git")
            .args(args)
            .current_dir(dir)
            .status()
            .expect("git runs");
        assert!(status.success(), "git {args:?}");
    }
}

#[test]
fn review_with_a_missing_base_names_the_ref() {
    let dir = tempdir().expect("tempdir");
    committed_repo(dir.path());

    let output = Command::new(env!("CARGO_BIN_EXE_repopilot"))
        .args(["review", ".", "--base", "origin/main"])
        .current_dir(dir.path())
        .output()
        .expect("repopilot runs");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(
        stderr.contains("Git ref `origin/main` was not found"),
        "{stderr}"
    );
    assert!(stderr.contains("git branch -a"), "{stderr}");
    assert!(!stderr.contains("fatal:"), "{stderr}");
}
