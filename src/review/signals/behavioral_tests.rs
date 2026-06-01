use super::behavioral::{BehavioralKind, detect_behavioral_added};
use super::content::ReviewSource;
use crate::review::diff::{ChangeStatus, ChangedFile, ChangedRange};
use std::path::PathBuf;

fn file_with_range(path: &str, status: ChangeStatus, start: usize, end: usize) -> ChangedFile {
    ChangedFile {
        path: PathBuf::from(path),
        status,
        ranges: vec![ChangedRange { start, end }],
        hunks: Vec::new(),
    }
}

#[test]
fn js_network_call_added_tp_fp() {
    let content = r#"
// Fetch call outside range
fetch("https://example.com/outside");

// Changed range starts here:
const data = await fetch("https://example.com/inside");
// Fetch call in a comment inside range:
// fetch("https://example.com/comment");
"#;

    let file = file_with_range("src/api.js", ChangeStatus::Modified, 5, 7);
    let source = ReviewSource::new(content.to_string(), Some("JavaScript".to_string()));

    let signals = detect_behavioral_added(&file, &source);

    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].kind, BehavioralKind::NetworkCallAdded);
    assert_eq!(signals[0].line, 6);
    assert!(signals[0].detail.contains("inside"));
}

#[test]
fn python_subprocess_and_env() {
    let content = r#"
import os
import subprocess

def run():
    # Inside range
    val = os.environ["API_KEY"]
    subprocess.run(["ls", "-l"])
"#;
    let file = file_with_range("app.py", ChangeStatus::Modified, 6, 8);
    let source = ReviewSource::new(content.to_string(), Some("Python".to_string()));

    let signals = detect_behavioral_added(&file, &source);

    let kinds: Vec<_> = signals.iter().map(|s| s.kind).collect();
    assert!(
        kinds.contains(&BehavioralKind::EnvVarIntroduced),
        "Expected EnvVarIntroduced in {:?}",
        kinds
    );
    assert!(
        kinds.contains(&BehavioralKind::SubprocessAdded),
        "Expected SubprocessAdded in {:?}",
        kinds
    );
}

#[test]
fn rust_fs_write_and_sql() {
    let content = r#"
use std::fs;

fn save() {
    // Inside range
    fs::write("data.txt", "hello").unwrap();
    let query = "SELECT name, age FROM users WHERE id = 1";
}
"#;
    let file = file_with_range("src/lib.rs", ChangeStatus::Modified, 5, 7);
    let source = ReviewSource::new(content.to_string(), Some("Rust".to_string()));

    let signals = detect_behavioral_added(&file, &source);

    let kinds: Vec<_> = signals.iter().map(|s| s.kind).collect();
    assert!(
        kinds.contains(&BehavioralKind::FsWriteAdded),
        "Expected FsWriteAdded in {:?}",
        kinds
    );
    assert!(
        kinds.contains(&BehavioralKind::RawSqlAdded),
        "Expected RawSqlAdded in {:?}",
        kinds
    );
}

#[test]
fn migration_added_path() {
    let file = ChangedFile {
        path: PathBuf::from("db/migrations/20260601_init.sql"),
        status: ChangeStatus::Added,
        ranges: vec![ChangedRange { start: 1, end: 1 }],
        hunks: Vec::new(),
    };
    let source = ReviewSource::new("".to_string(), None);
    let signals = detect_behavioral_added(&file, &source);
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].kind, BehavioralKind::MigrationAdded);
}
