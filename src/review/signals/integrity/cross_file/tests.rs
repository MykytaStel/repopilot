use super::*;
use crate::review::diff::{ChangeStatus, ChangedFile, ChangedRange};
use crate::review::signals::integrity::{IntegrityKind, collect_file_evidence};
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[path = "tests/python.rs"]
mod python;

#[test]
fn renamed_test_is_not_counted_as_an_unchanged_importer() {
    let repo = tempdir().expect("temporary repo");
    let root = repo.path();
    let helper_before = "def verify(value):\n    assert value\n    assert value is not None\n";
    let caller = "from src.checks import verify\n\ndef test_request():\n    verify(1)\n";
    write(root, "src/checks.py", helper_before);
    write(root, "tests/test_checks.py", caller);
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "test@example.invalid"]);
    git(root, &["config", "user.name", "RepoPilot test"]);
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "before"]);
    fs::remove_file(root.join("tests/test_checks.py")).expect("remove old test path");
    write(root, "src/tests/test_checks.py", caller);
    let helper_after = "def verify(value):\n    assert value\n";
    write(root, "src/checks.py", helper_after);
    git(root, &["add", "-A"]);

    let helper_file = ChangedFile {
        path: PathBuf::from("src/checks.py"),
        status: ChangeStatus::Modified,
        ranges: vec![ChangedRange { start: 1, end: 20 }],
        hunks: Vec::new(),
    };
    let moved_test_file = ChangedFile {
        path: PathBuf::from("src/tests/test_checks.py"),
        status: ChangeStatus::Renamed,
        ranges: Vec::new(),
        hunks: Vec::new(),
    };
    let helper_pre = ReviewSource::new(helper_before.to_string(), Some("Python".to_string()));
    let helper_post = ReviewSource::new(helper_after.to_string(), Some("Python".to_string()));
    let test_source = ReviewSource::new(caller.to_string(), Some("Python".to_string()));
    let changed = [
        ChangedIntegritySource {
            file: &helper_file,
            pre: Some(&helper_pre),
            post: Some(&helper_post),
        },
        ChangedIntegritySource {
            file: &moved_test_file,
            pre: Some(&test_source),
            post: Some(&test_source),
        },
    ];
    let evidence = collect_file_evidence(&helper_file, Some(&helper_pre), Some(&helper_post))
        .into_iter()
        .collect::<Vec<_>>();

    assert!(detect(root, DiffTarget::WorkingTree, &changed, &evidence).is_empty());
}

#[test]
fn go_same_package_changed_helper_is_counted_in_unchanged_test() {
    let before = "package app\nimport \"testing\"\nfunc Verify(t *testing.T) { t.Error(\"bad\"); t.Fatal(\"stop\") }\n";
    let after = "package app\nimport \"testing\"\nfunc Verify(t *testing.T) { t.Error(\"bad\") }\n";
    let caller = "package app\nimport \"testing\"\nfunc TestRequest(t *testing.T) { Verify(t) }\n";
    let signals =
        changed_helper_signals("verify.go", "Go", before, after, "verify_test.go", caller);

    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].kind, IntegrityKind::AssertionsRemoved);
    assert!(signals[0].path.ends_with("verify_test.go"));
    assert!(signals[0].detail.contains("verify.go::Verify"));
}

#[test]
fn go_local_function_value_shadow_does_not_match_package_helper() {
    let signals = changed_helper_signals(
        "verify.go",
        "Go",
        "package app\nimport \"testing\"\nfunc Verify(t *testing.T) { t.Error(\"bad\"); t.Fatal(\"stop\") }\n",
        "package app\nimport \"testing\"\nfunc Verify(t *testing.T) { t.Error(\"bad\") }\n",
        "verify_test.go",
        concat!(
            "package app\nimport \"testing\"\n",
            "func TestRequest(t *testing.T) {\n",
            "Verify := func(t *testing.T) { t.Error(\"local\") }\n",
            "Verify(t)\n",
            "}\n",
        ),
    );

    assert!(
        signals.is_empty(),
        "a test-local function value shadows the package helper"
    );
}

#[test]
fn go_cross_file_helper_does_not_cross_package_boundary() {
    let signals = changed_helper_signals(
        "verify.go",
        "Go",
        "package app\nimport \"testing\"\nfunc Verify(t *testing.T) { t.Error(\"bad\"); t.Fatal(\"stop\") }\n",
        "package app\nimport \"testing\"\nfunc Verify(t *testing.T) { t.Error(\"bad\") }\n",
        "verify_test.go",
        "package other\nimport \"testing\"\nfunc TestRequest(t *testing.T) { Verify(t) }\n",
    );
    assert!(signals.is_empty());
}

#[test]
fn go_unchanged_helper_checks_do_not_emit_integrity_signal() {
    let helper = "package app\nimport \"testing\"\nfunc Verify(t *testing.T) { t.Error(\"bad\"); t.Fatal(\"stop\") }\n";
    let signals = changed_helper_signals(
        "verify.go",
        "Go",
        helper,
        helper,
        "verify_test.go",
        "package app\nimport \"testing\"\nfunc TestRequest(t *testing.T) { Verify(t) }\n",
    );
    assert!(signals.is_empty(), "unchanged checks are not a regression");
}

#[test]
fn go_reordered_helper_checks_are_a_safe_refactor() {
    let signals = changed_helper_signals(
        "verify.go",
        "Go",
        "package app\nimport \"testing\"\nfunc Verify(t *testing.T) { t.Error(\"bad\"); t.Fatal(\"stop\") }\n",
        "package app\nimport \"testing\"\nfunc Verify(t *testing.T) { t.Fatal(\"stop\"); t.Error(\"bad\") }\n",
        "verify_test.go",
        "package app\nimport \"testing\"\nfunc TestRequest(t *testing.T) { Verify(t) }\n",
    );
    assert!(signals.is_empty(), "reordering checks keeps their meaning");
}

#[test]
fn go_unrelated_test_in_same_package_is_not_attributed() {
    let signals = changed_helper_signals(
        "verify.go",
        "Go",
        "package app\nimport \"testing\"\nfunc Verify(t *testing.T) { t.Error(\"bad\"); t.Fatal(\"stop\") }\n",
        "package app\nimport \"testing\"\nfunc Verify(t *testing.T) { t.Error(\"bad\") }\n",
        "other_test.go",
        "package app\nimport \"testing\"\nfunc TestOther(t *testing.T) { t.Log(\"unrelated\") }\n",
    );
    assert!(
        signals.is_empty(),
        "same-package tests must call the changed helper"
    );
}

#[test]
fn go_ambiguous_changed_helper_declarations_are_not_attributed() {
    let before = concat!(
        "package app\nimport \"testing\"\n",
        "func Verify(t *testing.T) { t.Error(\"bad\"); t.Fatal(\"stop\") }\n",
        "func Verify(t *testing.T) { t.Error(\"other\"); t.Fatal(\"stop\") }\n",
    );
    let after = concat!(
        "package app\nimport \"testing\"\n",
        "func Verify(t *testing.T) { t.Error(\"bad\") }\n",
        "func Verify(t *testing.T) { t.Error(\"other\"); t.Fatal(\"stop\") }\n",
    );
    let signals = changed_helper_signals(
        "verify.go",
        "Go",
        before,
        after,
        "verify_test.go",
        "package app\nimport \"testing\"\nfunc TestRequest(t *testing.T) { Verify(t) }\n",
    );
    assert!(signals.is_empty(), "ambiguous targets must be withheld");
}

fn changed_helper_signals(
    helper_path: &str,
    language: &str,
    before_text: &str,
    after_text: &str,
    caller_path: &str,
    caller_text: &str,
) -> Vec<IntegritySignal> {
    let repo = tempdir().expect("temporary repo");
    let root = repo.path();
    write(root, helper_path, before_text);
    write(root, caller_path, caller_text);
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "test@example.invalid"]);
    git(root, &["config", "user.name", "RepoPilot test"]);
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "before"]);
    write(root, helper_path, after_text);

    let file = ChangedFile {
        path: PathBuf::from(helper_path),
        status: ChangeStatus::Modified,
        ranges: vec![ChangedRange { start: 1, end: 100 }],
        hunks: Vec::new(),
    };
    let before = ReviewSource::new(before_text.to_string(), Some(language.to_string()));
    let after = ReviewSource::new(after_text.to_string(), Some(language.to_string()));
    let changed = [ChangedIntegritySource {
        file: &file,
        pre: Some(&before),
        post: Some(&after),
    }];
    let evidence = collect_file_evidence(&file, Some(&before), Some(&after))
        .into_iter()
        .collect::<Vec<_>>();
    detect(root, DiffTarget::WorkingTree, &changed, &evidence)
}

fn write(root: &Path, path: &str, content: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(root)
        .status()
        .expect("git runs");
    assert!(status.success(), "git {args:?}");
}
