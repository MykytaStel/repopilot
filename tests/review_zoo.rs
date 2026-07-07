//! Review-zoo: differential safe/unsafe fixtures for the `review` change signals.
//!
//! A companion to `tests/review_golden_fixtures.rs`'s golden harness. Where
//! that harness proves one true-positive fixture per signal family, this one
//! proves the *precision boundary*: for the same delta family, a `safe/`
//! variant that must produce zero review signals sits next to an `unsafe/`
//! variant that must produce the expected signal.
//!
//! Each scenario lives at `tests/fixtures/review-zoo/<family>/<scenario>/`
//! with a `safe/` and an `unsafe/` subdirectory, each shaped like a golden
//! fixture: `before/` (committed baseline), `after/` (overlaid, left
//! uncommitted — the diff review reads), `patch/rationale.md` (required,
//! non-empty — why this edit is/isn't expected to signal), and
//! `expected.json` (`{ description, expect[] }`, `expect` only required for
//! `unsafe/`). See `tests/fixtures/review-zoo/README.md` for the full format.

use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

fn zoo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/review-zoo")
}

#[test]
fn review_zoo_covers_boundary_behavioral_and_taint_families() {
    let required: BTreeSet<&str> = ["boundary", "behavioral", "taint"].into_iter().collect();
    let found: BTreeSet<String> = sorted_subdirs(&zoo_root())
        .iter()
        .filter_map(|path| path.file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .collect();
    let missing: Vec<&&str> = required
        .iter()
        .filter(|family| !found.contains(**family))
        .collect();
    assert!(
        missing.is_empty(),
        "review-zoo must cover at least {required:?}, missing {missing:?} (found {found:?})"
    );
}

#[test]
fn review_zoo_safe_and_unsafe_fixtures_match_expected_signals() {
    let root = zoo_root();
    let mut scenarios = Vec::new();
    for family in sorted_subdirs(&root) {
        scenarios.extend(sorted_subdirs(&family));
    }
    assert!(
        scenarios.len() >= 3,
        "expected at least 3 review-zoo scenarios, found {}",
        scenarios.len()
    );

    for scenario in scenarios {
        for variant in ["safe", "unsafe"] {
            let dir = scenario.join(variant);
            assert!(
                dir.is_dir(),
                "{}: missing required `{variant}/` variant",
                scenario.display()
            );
            run_variant(&dir, variant);
        }
    }
}

fn run_variant(dir: &Path, variant: &str) {
    let name = dir
        .strip_prefix(Path::new(env!("CARGO_MANIFEST_DIR")))
        .unwrap_or(dir)
        .display()
        .to_string();

    assert_rationale_present(dir, &name);

    let temp = tempdir().expect("temp dir");
    let repo = temp.path();
    git(repo, &["init"]);
    git(repo, &["config", "user.email", "repopilot@example.invalid"]);
    git(repo, &["config", "user.name", "RepoPilot Test"]);

    copy_tree(&dir.join("before"), repo);
    git(repo, &["add", "."]);
    git(repo, &["commit", "-m", "before"]);

    let after = dir.join("after");
    if after.is_dir() {
        copy_tree(&after, repo);
    }

    let output = repopilot()
        .args(["review", ".", "--format", "json"])
        .current_dir(repo)
        .output()
        .unwrap_or_else(|err| panic!("[{name}] failed to run review: {err}"));
    assert!(
        output.status.success(),
        "[{name}] review exited with {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|err| panic!("[{name}] review did not emit JSON: {err}"));

    let signals = flatten_signals(&json);

    match variant {
        "safe" => {
            assert!(
                signals.is_empty(),
                "[{name}] safe fixture must produce zero review signals\nobserved:\n{}",
                describe(&signals)
            );
            let boundary_signals = json["boundary_signals"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            assert!(
                boundary_signals.is_empty(),
                "[{name}] safe fixture must produce zero boundary_signals, got {boundary_signals:?}"
            );
        }
        "unsafe" => {
            let expected = read_expected(dir, &name);
            let expectations = expected["expect"].as_array().cloned().unwrap_or_default();
            assert!(
                !expectations.is_empty(),
                "[{name}] unsafe fixture's expected.json must declare at least one `expect` constraint"
            );
            for constraint in &expectations {
                assert!(
                    signals
                        .iter()
                        .any(|signal| signal_matches(signal, constraint)),
                    "[{name}] no signal matched expect {constraint}\nobserved:\n{}",
                    describe(&signals)
                );
            }
        }
        other => panic!("unknown variant {other}"),
    }
}

fn assert_rationale_present(dir: &Path, name: &str) {
    let path = dir.join("patch/rationale.md");
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("[{name}] missing patch/rationale.md: {err}"));
    assert!(
        !text.trim().is_empty(),
        "[{name}] patch/rationale.md must not be empty"
    );
}

/// All tiered signals, each tagged with its `bucket` (`definitely`/`maybe`/`noise`).
fn flatten_signals(json: &Value) -> Vec<Value> {
    let mut out = Vec::new();
    for bucket in ["definitely", "maybe", "noise"] {
        for signal in json["tiered_signals"][bucket]
            .as_array()
            .into_iter()
            .flatten()
        {
            let mut signal = signal.clone();
            if let Some(object) = signal.as_object_mut() {
                object.insert("bucket".to_string(), Value::String(bucket.to_string()));
            }
            out.push(signal);
        }
    }
    out
}

/// Partial match: every field named in `constraint` must equal the signal's field.
fn signal_matches(signal: &Value, constraint: &Value) -> bool {
    let Some(fields) = constraint.as_object() else {
        return false;
    };
    fields
        .iter()
        .all(|(key, value)| signal.get(key) == Some(value))
}

fn read_expected(dir: &Path, name: &str) -> Value {
    let raw = fs::read_to_string(dir.join("expected.json"))
        .unwrap_or_else(|err| panic!("[{name}] missing expected.json: {err}"));
    serde_json::from_str(&raw).unwrap_or_else(|err| panic!("[{name}] invalid expected.json: {err}"))
}

fn describe(signals: &[Value]) -> String {
    if signals.is_empty() {
        return "  (no signals)".to_string();
    }
    signals
        .iter()
        .map(|signal| {
            format!(
                "  - bucket={} family={} kind={} path={} headline={}",
                signal["bucket"],
                signal["family"],
                signal["kind"],
                signal["path"],
                signal["headline"]
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn sorted_subdirs(dir: &Path) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|err| panic!("read {}: {err}", dir.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    dirs.sort();
    dirs
}

fn copy_tree(src: &Path, dest: &Path) {
    for entry in fs::read_dir(src).unwrap_or_else(|err| panic!("read {}: {err}", src.display())) {
        let entry = entry.expect("dir entry");
        let from = entry.path();
        let to = dest.join(entry.file_name());
        if from.is_dir() {
            fs::create_dir_all(&to).expect("create dir");
            copy_tree(&from, &to);
        } else {
            if let Some(parent) = to.parent() {
                fs::create_dir_all(parent).expect("create parent");
            }
            fs::copy(&from, &to).expect("copy file");
        }
    }
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
