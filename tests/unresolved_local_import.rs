use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

const RULE_ID: &str = "architecture.unresolved-local-import";

#[test]
fn full_and_warm_changed_scans_preserve_broken_import_evidence() {
    let temp = tempdir().expect("temp dir");
    init_repo(temp.path());
    write(
        &temp.path().join("src/app.ts"),
        "// application entry\nimport { run } from \"./present.ts\";\nrun();\n",
    );
    write(
        &temp.path().join("src/present.ts"),
        "export function run() {}\n",
    );
    commit_all(temp.path(), "initial");
    write(
        &temp.path().join("src/app.ts"),
        "// application entry\nimport { run } from \"./missing.ts\";\nrun();\n",
    );

    let full = scan_json(temp.path(), &[]);
    let _cold_changed = scan_json(temp.path(), &["--changed"]);
    let warm_changed = scan_json(temp.path(), &["--changed"]);
    let full_finding = rule_finding(&full);
    let changed_finding = rule_finding(&warm_changed);

    assert_eq!(full_finding["rule_id"], RULE_ID);
    assert_eq!(full_finding["severity"], "HIGH");
    assert_eq!(full_finding["confidence"], "HIGH");
    assert_eq!(full_finding["evidence"][0]["path"], "src/app.ts");
    assert_eq!(full_finding["evidence"][0]["line_start"], 2);
    assert_eq!(full_finding["evidence"][0], changed_finding["evidence"][0]);
    assert_eq!(full_finding["id"], changed_finding["id"]);
}

#[test]
fn full_and_warm_changed_scans_ignore_guarded_optional_python_import() {
    let temp = tempdir().expect("temp dir");
    init_repo(temp.path());
    write(&temp.path().join("pkg/__init__.py"), "");
    write(&temp.path().join("pkg/settings.py"), "VALUE = 1\n");
    commit_all(temp.path(), "initial");
    write(
        &temp.path().join("pkg/settings.py"),
        "try:\n    from .local import *\nexcept ImportError:\n    pass\nVALUE = 1\n",
    );

    let full = scan_json(temp.path(), &[]);
    let cold_changed = scan_json(temp.path(), &["--changed"]);
    let warm_changed = scan_json(temp.path(), &["--changed"]);

    assert_rule_absent(&full);
    assert_rule_absent(&cold_changed);
    assert_rule_absent(&warm_changed);
    assert!(
        warm_changed["cache_telemetry"]["parsed_cache_hits"]
            .as_u64()
            .is_some_and(|hits| hits > 0),
        "warm changed scan should restore guarded facts from cache: {warm_changed:#?}"
    );
}

#[test]
fn reraising_import_error_keeps_missing_python_import_finding() {
    let temp = tempdir().expect("temp dir");
    init_repo(temp.path());
    write(&temp.path().join("pkg/__init__.py"), "");
    write(
        &temp.path().join("pkg/settings.py"),
        "try:\n    from .missing import run\nexcept ImportError:\n    raise\n",
    );

    let full = scan_json(temp.path(), &[]);

    assert_eq!(
        rule_finding(&full)["evidence"][0]["path"],
        "pkg/settings.py"
    );
}

fn scan_json(root: &Path, extra: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_repopilot"))
        .arg("scan")
        .arg(root)
        .args(extra)
        .args(["--format", "json"])
        .output()
        .expect("run RepoPilot scan");
    assert!(
        output.status.success(),
        "scan failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("valid JSON scan report")
}

fn rule_finding(report: &Value) -> &Value {
    report["findings"]
        .as_array()
        .expect("findings array")
        .iter()
        .find(|finding| finding["rule_id"] == RULE_ID)
        .expect("broken local import finding")
}

fn assert_rule_absent(report: &Value) {
    assert!(
        report["findings"]
            .as_array()
            .expect("findings array")
            .iter()
            .all(|finding| finding["rule_id"] != RULE_ID),
        "guarded optional import should not produce {RULE_ID}: {report:#?}"
    );
}

fn init_repo(root: &Path) {
    run_git(root, &["init", "-q"]);
    run_git(root, &["config", "user.email", "test@example.com"]);
    run_git(root, &["config", "user.name", "RepoPilot Test"]);
}

fn commit_all(root: &Path, message: &str) {
    run_git(root, &["add", "."]);
    run_git(root, &["commit", "-q", "-m", message]);
}

fn run_git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(root)
        .status()
        .expect("run git");
    assert!(status.success(), "git command failed: {args:?}");
}

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().expect("file parent")).unwrap();
    fs::write(path, content).unwrap();
}
