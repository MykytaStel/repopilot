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

#[test]
fn pagination_does_not_change_full_report_cache_key() {
    let (_temp, root) = init_repo();
    let initial = args(&root);
    let mut page = initial.clone();
    page["offset"] = json!(10);
    page["limit"] = json!(20);
    let first = cache_key(&root, &initial).unwrap();
    assert_eq!(cache_key(&root, &page).as_deref(), Some(first.as_str()));
    page["profile"] = json!("strict");
    assert_ne!(cache_key(&root, &page).as_deref(), Some(first.as_str()));
}

#[test]
fn same_worktree_bytes_with_different_staged_blobs_invalidate_cache() {
    let (_temp, root) = init_repo();
    let file = root.join("src/lib.rs");
    fs::write(&file, "pub fn staged_one() {}\n").unwrap();
    git(&root, &["add", "src/lib.rs"]);
    fs::write(&file, "pub fn worktree() {}\n").unwrap();
    let first = cache_key(&root, &args(&root)).unwrap();
    fs::write(&file, "pub fn staged_two() {}\n").unwrap();
    git(&root, &["add", "src/lib.rs"]);
    fs::write(&file, "pub fn worktree() {}\n").unwrap();
    let second = cache_key(&root, &args(&root)).unwrap();
    assert_ne!(first, second);
}

#[cfg(unix)]
#[test]
fn dirty_symlink_disables_disk_cache() {
    use std::os::unix::fs::symlink;

    let (_temp, root) = init_repo();
    let outside = tempfile::tempdir().unwrap();
    let target = outside.path().join("external.rs");
    fs::write(&target, "pub fn external() {}\n").unwrap();
    symlink(&target, root.join("src/link.rs")).unwrap();
    assert!(cache_key(&root, &args(&root)).is_none());
}
