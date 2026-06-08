use super::behavioral::{
    BehavioralKind, BehavioralSignalSource, DependencyContext, detect_behavioral_added,
    detect_behavioral_removed,
};
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

    let signals = detect_behavioral_added(&file, &source, &DependencyContext::default());

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

    let signals = detect_behavioral_added(&file, &source, &DependencyContext::default());

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

    let signals = detect_behavioral_added(&file, &source, &DependencyContext::default());

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
fn standard_library_and_local_imports_are_not_dependency_signals() {
    let node = ReviewSource::new(
        "import * as path from \"node:path\";\n".to_string(),
        Some("TypeScript".to_string()),
    );
    let node_file = file_with_range("src/path.ts", ChangeStatus::Modified, 1, 1);
    assert!(detect_behavioral_added(&node_file, &node, &DependencyContext::default()).is_empty());

    let python = ReviewSource::new(
        "import argparse\nfrom pathlib import Path\n".to_string(),
        Some("Python".to_string()),
    );
    let python_file = file_with_range("tools/check.py", ChangeStatus::Modified, 1, 2);
    assert!(
        detect_behavioral_added(&python_file, &python, &DependencyContext::default()).is_empty()
    );

    let temp = tempfile::tempdir().expect("temp repo");
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"demo-core\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("write Cargo.toml");
    let manifest = std::fs::read_to_string(temp.path().join("Cargo.toml")).expect("read manifest");
    let parsed = toml::from_str::<toml::Value>(&manifest).expect("parse manifest");
    assert_eq!(parsed["package"]["name"].as_str(), Some("demo-core"));
    let rust = ReviewSource::new(
        "mod lang;\npub mod diff;\nuse std::path::Path;\nuse demo_core::review::run;\nuse lang::parse;\npub use diff::Report;\n"
            .to_string(),
        Some("Rust".to_string()),
    );
    let rust_file = file_with_range("src/main.rs", ChangeStatus::Modified, 1, 6);
    let dependencies = DependencyContext::from_repo_root(temp.path());
    assert!(
        dependencies.is_local_package("demo_core"),
        "{dependencies:?}"
    );
    let rust_signals = detect_behavioral_added(&rust_file, &rust, &dependencies);
    assert!(rust_signals.is_empty(), "{rust_signals:?}");
}

#[test]
fn runtime_side_effects_in_test_files_are_not_review_signals() {
    let content = r#"
use std::fs;
use std::process::Command;

#[test]
fn writes_fixture() {
    fs::write("fixture.txt", "demo").unwrap();
    Command::new("echo").arg("demo").status().unwrap();
}
"#;
    let source = ReviewSource::new(content.to_string(), Some("Rust".to_string()));

    for path in [
        "tests/fixture_builder.rs",
        "src/behavioral_tests.rs",
        "fixtures/runtime/fixture_builder.rs",
    ] {
        let file = file_with_range(path, ChangeStatus::Modified, 1, 9);
        assert!(
            detect_behavioral_added(&file, &source, &DependencyContext::default()).is_empty(),
            "unexpected side-effect signal for {path}"
        );
    }
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
    let signals = detect_behavioral_added(&file, &source, &DependencyContext::default());
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].kind, BehavioralKind::MigrationAdded);
}

fn file_with_hunk(
    path: &str,
    status: ChangeStatus,
    old_range: Option<(usize, usize)>,
    new_range: Option<(usize, usize)>,
    removed_lines: Vec<&str>,
    added_lines: Vec<&str>,
) -> ChangedFile {
    use crate::review::diff::DiffHunk;
    let old_range = old_range.map(|(start, end)| ChangedRange { start, end });
    let new_range = new_range.map(|(start, end)| ChangedRange { start, end });
    ChangedFile {
        path: PathBuf::from(path),
        status,
        ranges: new_range.map(|r| vec![r]).unwrap_or_default(),
        hunks: vec![DiffHunk {
            new_range,
            old_range,
            added_lines: added_lines.into_iter().map(String::from).collect(),
            removed_lines: removed_lines.into_iter().map(String::from).collect(),
        }],
    }
}

#[test]
fn test_deleted_or_emptied() {
    // Test Deleted:
    let file_del = ChangedFile {
        path: PathBuf::from("src/app.test.js"),
        status: ChangeStatus::Deleted,
        ranges: Vec::new(),
        hunks: Vec::new(),
    };
    let signals_del = detect_behavioral_removed(&file_del, None, None);
    assert_eq!(signals_del.len(), 1);
    assert_eq!(signals_del[0].kind, BehavioralKind::TestDeletedOrEmptied);

    // Test Emptied:
    let file_empty = file_with_hunk(
        "src/app.test.js",
        ChangeStatus::Modified,
        Some((1, 3)),
        Some((1, 1)),
        vec!["test('foo', () => {});"],
        vec![""],
    );
    let pre_src = ReviewSource::new(
        "test('foo', () => {});".to_string(),
        Some("JavaScript".to_string()),
    );
    let post_src = ReviewSource::new("".to_string(), Some("JavaScript".to_string()));
    let signals_empty = detect_behavioral_removed(&file_empty, Some(&pre_src), Some(&post_src));
    assert_eq!(signals_empty.len(), 1);
    assert_eq!(signals_empty[0].kind, BehavioralKind::TestDeletedOrEmptied);
}

#[test]
fn try_catch_removed() {
    let pre_code = r#"
try {
    doSomething();
} catch (err) {
    console.error(err);
}
"#;
    let post_code = r#"
doSomething();
"#;
    let file = file_with_hunk(
        "src/main.js",
        ChangeStatus::Modified,
        Some((2, 6)),
        Some((2, 2)),
        vec![
            "try {",
            "    doSomething();",
            "} catch (err) {",
            "    console.error(err);",
            "}",
        ],
        vec!["doSomething();"],
    );
    let pre_src = ReviewSource::new(pre_code.to_string(), Some("JavaScript".to_string()));
    let post_src = ReviewSource::new(post_code.to_string(), Some("JavaScript".to_string()));

    let signals = detect_behavioral_removed(&file, Some(&pre_src), Some(&post_src));
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].kind, BehavioralKind::ErrorHandlingRemoved);
}

#[test]
fn coarse_fallback_uses_token_matching_no_ast() {
    // With no sources the AST path is skipped, so the coarse fallback scans raw
    // hunk lines. Real auth/error tokens fire and are marked coarse.
    let file = file_with_hunk(
        "src/legacy.php",
        ChangeStatus::Modified,
        Some((1, 4)),
        Some((1, 2)),
        vec![
            "} catch (Exception $e) {",
            "if (!isAuthenticated($req)) { abort(401); }",
        ],
        vec![],
    );
    let signals = detect_behavioral_removed(&file, None, None);
    let kinds: Vec<_> = signals.iter().map(|s| s.kind).collect();
    assert!(
        kinds.contains(&BehavioralKind::ErrorHandlingRemoved),
        "expected ErrorHandlingRemoved in {kinds:?}"
    );
    assert!(
        kinds.contains(&BehavioralKind::AuthCheckRemoved),
        "expected AuthCheckRemoved in {kinds:?}"
    );
    assert!(
        signals
            .iter()
            .all(|signal| signal.source == BehavioralSignalSource::CoarseFallback),
        "coarse fallback signals must carry the CoarseFallback source: {signals:?}"
    );
    assert!(
        signals.iter().all(|signal| signal.is_coarse()),
        "is_coarse() must be true for CoarseFallback signals: {signals:?}"
    );
}

#[test]
fn coarse_fallback_ignores_lookalike_tokens() {
    // `author` and `catchy` must not be mistaken for `auth` and `catch`.
    let file = file_with_hunk(
        "src/legacy.php",
        ChangeStatus::Modified,
        Some((1, 3)),
        Some((1, 1)),
        vec!["$author = $post->author;", "$catchy = makeCatcher();"],
        vec![],
    );
    let signals = detect_behavioral_removed(&file, None, None);
    assert!(
        signals.is_empty(),
        "lookalike tokens (author, catchy) must not fire coarse signals: {signals:?}"
    );
}

#[test]
fn prose_files_do_not_use_removed_behavior_fallbacks() {
    let file = file_with_hunk(
        "docs/security.md",
        ChangeStatus::Modified,
        Some((1, 2)),
        Some((1, 1)),
        vec![
            "Authentication is checked before every request.",
            "Handle errors explicitly.",
        ],
        vec![],
    );

    assert!(detect_behavioral_removed(&file, None, None).is_empty());
}

#[test]
fn auth_check_removed() {
    let pre_code = r#"
function run() {
    checkPermission("admin");
    doAction();
}
"#;
    let post_code = r#"
function run() {
    doAction();
}
"#;
    let file = file_with_hunk(
        "src/main.js",
        ChangeStatus::Modified,
        Some((3, 3)),
        Some((3, 3)),
        vec!["    checkPermission(\"admin\");"],
        vec![],
    );
    let pre_src = ReviewSource::new(pre_code.to_string(), Some("JavaScript".to_string()));
    let post_src = ReviewSource::new(post_code.to_string(), Some("JavaScript".to_string()));

    let signals = detect_behavioral_removed(&file, Some(&pre_src), Some(&post_src));
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].kind, BehavioralKind::AuthCheckRemoved);
    // AST-confirmed signals carry the Ast source and are not coarse.
    assert_eq!(signals[0].source, BehavioralSignalSource::Ast);
    assert!(!signals[0].is_coarse());
}
