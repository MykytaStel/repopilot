use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;

pub(super) const RULE_ID: &str = "behavioral.removed-export-still-imported";

pub(super) fn finding_for_rule(report: &Value) -> &Value {
    report["findings"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|finding| finding["rule_id"] == RULE_ID)
        .unwrap_or_else(|| panic!("missing {RULE_ID}: {report:#?}"))
}

pub(super) fn assert_rule_absent(report: &Value) {
    assert!(
        report["findings"]
            .as_array()
            .into_iter()
            .flatten()
            .all(|finding| finding["rule_id"] != RULE_ID),
        "unexpected {RULE_ID}: {report:#?}",
    );
}

pub(super) fn remove_caller_symbol_facts(root: &Path) {
    let path = root.join(".repopilot/cache/parsed_facts_v2.json");
    let mut cache: Value =
        serde_json::from_slice(&fs::read(&path).expect("read parsed cache")).expect("cache JSON");
    cache["entries"]
        .as_array_mut()
        .expect("cache entries")
        .retain(|entry| {
            entry["javascript_symbols"]["imports"]
                .as_array()
                .is_none_or(Vec::is_empty)
        });
    fs::write(
        path,
        serde_json::to_vec_pretty(&cache).expect("render parsed cache"),
    )
    .expect("rewrite parsed cache");
}

pub(super) fn scan_json(root: &Path, args: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_repopilot"))
        .args(["scan", ".", "--format", "json"])
        .args(args)
        .current_dir(root)
        .output()
        .expect("run scan");
    assert!(
        output.status.success(),
        "scan failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    serde_json::from_slice(&output.stdout).expect("scan JSON")
}

pub(super) fn init_repo(root: &Path) {
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "test@example.com"]);
    git(root, &["config", "user.name", "RepoPilot Test"]);
}

pub(super) fn write(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
    fs::write(path, content).expect("write source");
}

pub(super) fn commit_all(root: &Path, message: &str) {
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", message]);
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
        String::from_utf8_lossy(&output.stderr),
    );
}

pub(super) fn git_stdout(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("run git");
    assert!(output.status.success(), "git {args:?} failed");
    String::from_utf8(output.stdout)
        .expect("git UTF-8")
        .trim()
        .to_string()
}
