use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

#[test]
fn vibe_rejects_unknown_focus() {
    let temp = tempdir().expect("failed to create temp dir");
    fs::write(temp.path().join("lib.rs"), "fn main() {}\n").expect("failed to write file");

    let output = repopilot()
        .args(["vibe", ".", "--focus", "securty"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot vibe");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(stderr.contains("invalid value"));
    assert!(stderr.contains("security"));
}

#[test]
fn vibe_rejects_unknown_budget() {
    let temp = tempdir().expect("failed to create temp dir");
    fs::write(temp.path().join("lib.rs"), "fn main() {}\n").expect("failed to write file");

    let output = repopilot()
        .args(["vibe", ".", "--budget", "banana"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot vibe");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(stderr.contains("positive token count"));
}

#[test]
fn vibe_rejects_zero_budget() {
    let temp = tempdir().expect("failed to create temp dir");
    fs::write(temp.path().join("lib.rs"), "fn main() {}\n").expect("failed to write file");

    let output = repopilot()
        .args(["vibe", ".", "--budget", "0"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run repopilot vibe");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(stderr.contains("greater than zero"));
}
