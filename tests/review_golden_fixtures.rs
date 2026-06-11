//! Golden harness for the `review` change signals.
//!
//! Unit tests next to each detector prove the true/false-positive rules in
//! isolation. This harness proves the signals survive the *real* pipeline: a
//! git diff parsed from a temp repo, folded into the unified tiered view, and
//! serialized through `repopilot review --format json` (the agent-facing
//! `repopilot_review_change` shape).
//!
//! Each fixture lives at `tests/fixtures/review/<family>/<scenario>/` with:
//!   - `before/` — a file tree committed as the baseline (HEAD),
//!   - `after/`  — a file tree overlaid on top and left uncommitted, so review
//!     sees it as the working-tree diff (new files are added, shared paths are
//!     overwritten — the seed slice does not model deletions),
//!   - `expected.json` — `{ "expect": [..], "forbid": [..] }` of partial-match
//!     constraints over the tiered signals. Every `expect` entry must match at
//!     least one emitted signal; every `forbid` entry must match none.
//!
//! Matching is on stable fields only (`bucket`, `family`, `kind`, `path`,
//! `headline`) — never timing, ids, or blast radius — so fixtures stay robust
//! across unrelated output changes. Add a fixture by dropping a directory in;
//! the harness discovers it, no registration needed.

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

fn repopilot() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
}

#[test]
fn review_golden_fixtures_match_expected_signals() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/review");
    let mut scenarios = Vec::new();
    for family in sorted_subdirs(&root) {
        scenarios.extend(sorted_subdirs(&family));
    }
    assert!(
        scenarios.len() >= 4,
        "expected at least the four seed fixtures under {}, found {}",
        root.display(),
        scenarios.len()
    );
    for scenario in scenarios {
        run_fixture(&scenario);
    }
}

fn run_fixture(scenario: &Path) {
    let name = scenario
        .strip_prefix(Path::new(env!("CARGO_MANIFEST_DIR")))
        .unwrap_or(scenario)
        .display()
        .to_string();

    let temp = tempdir().expect("temp dir");
    let repo = temp.path();
    git(repo, &["init"]);
    git(repo, &["config", "user.email", "repopilot@example.invalid"]);
    git(repo, &["config", "user.name", "RepoPilot Test"]);

    copy_tree(&scenario.join("before"), repo);
    git(repo, &["add", "."]);
    git(repo, &["commit", "-m", "before"]);

    // Overlay the post-change tree; left uncommitted it *is* the diff review reads.
    copy_tree(&scenario.join("after"), repo);

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
    let expected = read_expected(scenario, &name);

    for constraint in expected["expect"].as_array().into_iter().flatten() {
        assert!(
            signals
                .iter()
                .any(|signal| signal_matches(signal, constraint)),
            "[{name}] no signal matched expect {constraint}\nobserved:\n{}",
            describe(&signals)
        );
    }
    for constraint in expected["forbid"].as_array().into_iter().flatten() {
        assert!(
            !signals
                .iter()
                .any(|signal| signal_matches(signal, constraint)),
            "[{name}] a signal matched forbid {constraint}\nobserved:\n{}",
            describe(&signals)
        );
    }
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

fn read_expected(scenario: &Path, name: &str) -> Value {
    let raw = fs::read_to_string(scenario.join("expected.json"))
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
