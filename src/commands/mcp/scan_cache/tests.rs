use super::*;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

mod changed_base;
mod invalidation;
mod storage_tests;

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .expect("git is available");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_stdout(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .expect("git is available");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn init_repo() -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path().to_path_buf();
    git(&root, &["init", "-q"]);
    git(&root, &["config", "user.email", "t@example.com"]);
    git(&root, &["config", "user.name", "Test"]);
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 { a + b }\n",
    )
    .unwrap();
    git(&root, &["add", "."]);
    git(&root, &["commit", "-qm", "init"]);
    (dir, root)
}

fn args(root: &Path) -> Value {
    json!({ "path": root.to_str().unwrap() })
}

fn changed_args(root: &Path, base: &str) -> Value {
    json!({ "path": root.to_str().unwrap(), "scope": "changed", "base": base })
}

fn cached_scan_probe(marker: &str) -> String {
    json!({
        "schema_version": "test",
        "report": { "kind": "scan" },
        "cache_probe": marker
    })
    .to_string()
}

fn cache_dir_for(root: &Path) -> PathBuf {
    storage::cache_dir(root).expect("safe git-owned cache dir")
}
