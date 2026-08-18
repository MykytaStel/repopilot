use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

pub(super) const B21_KIND: &str = "behavioral.removed-export-still-imported";

pub(super) fn assert_canonical_occurrence(signal: &Value) {
    assert_eq!(signal["kind"], B21_KIND);
    assert_eq!(signal["family"], "behavioral");
    assert_eq!(signal["tier"], "definitely-sensitive");
    assert_eq!(signal["path"], "src/caller.ts");
    assert_eq!(signal["target_path"], "src/api.ts");
    assert_eq!(signal["headline"], "removed export is still imported");
    assert_eq!(signal["gate_eligible"], true);
}

pub(super) fn only_b21(report: &Value) -> &Value {
    let records = report["tiered_signals"]["definitely"]
        .as_array()
        .expect("definitely signals")
        .iter()
        .filter(|signal| signal["kind"] == B21_KIND)
        .collect::<Vec<_>>();
    assert_eq!(records.len(), 1, "{report:#?}");
    records[0]
}

pub(super) fn b21_records(report: &Value) -> Vec<Value> {
    report["tiered_signals"]["definitely"]
        .as_array()
        .expect("definitely signals")
        .iter()
        .filter(|signal| signal["kind"] == B21_KIND)
        .cloned()
        .collect()
}

pub(super) fn run_review_json(root: &Path, args: &[&str]) -> Value {
    let output = run_review(root, args);
    assert!(
        output.status.success(),
        "review failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("review JSON")
}

pub(super) fn run_review(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_repopilot"))
        .args(args)
        .current_dir(root)
        .output()
        .expect("run repopilot review")
}

pub(super) fn init_repo(root: &Path) {
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "repopilot@example.invalid"]);
    git(root, &["config", "user.name", "RepoPilot Test"]);
}

pub(super) fn write(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent directory")).expect("create parent");
    fs::write(path, content).expect("write source");
}

pub(super) fn commit_all(root: &Path, message: &str) {
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", message]);
}

pub(super) fn git(root: &Path, args: &[&str]) {
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

pub(super) fn git_stdout(root: &Path, args: &[&str]) -> String {
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
    String::from_utf8(output.stdout)
        .expect("git stdout is UTF-8")
        .trim()
        .to_string()
}
