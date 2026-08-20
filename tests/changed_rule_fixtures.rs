use repopilot::rules::lookup_rule_metadata;
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Deserialize)]
struct FixtureManifest {
    fixtures: Vec<FixtureCase>,
}

#[derive(Deserialize)]
struct FixtureCase {
    path: String,
    expected_rule_ids: Vec<String>,
}

#[test]
fn changed_rule_fixtures_are_registered_deterministic_and_contract_clean() {
    let root = fixture_root();
    let rule_dirs = fs::read_dir(&root)
        .unwrap_or_else(|error| panic!("read {}: {error}", root.display()))
        .map(|entry| entry.expect("fixture entry").path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    assert!(
        !rule_dirs.is_empty(),
        "changed-rule fixture corpus is empty"
    );

    for rule_dir in rule_dirs {
        let rule_id = rule_dir
            .file_name()
            .and_then(|name| name.to_str())
            .expect("UTF-8 rule fixture name");
        assert!(
            lookup_rule_metadata(rule_id).is_some(),
            "unknown changed-rule fixture directory {rule_id}",
        );
        evaluate_rule_fixtures(&rule_dir, rule_id);
    }
}

fn evaluate_rule_fixtures(rule_dir: &Path, rule_id: &str) {
    let manifest: FixtureManifest = serde_json::from_slice(
        &fs::read(rule_dir.join("expected.json")).expect("read fixture manifest"),
    )
    .expect("fixture manifest JSON");
    for case in manifest.fixtures {
        let case_root = rule_dir.join(&case.path);
        let temp = tempfile::tempdir().expect("temp repo");
        copy_tree(&case_root.join("before"), temp.path());
        init_repo(temp.path());
        commit_all(temp.path(), "before");
        copy_tree(&case_root.join("after"), temp.path());

        let cold = scan_changed(temp.path());
        let warm = scan_changed(temp.path());
        let cold_records = findings_for_rule(&cold, rule_id);
        let warm_records = findings_for_rule(&warm, rule_id);
        assert_eq!(cold_records, warm_records, "warm drift in {}", case.path);
        assert_eq!(
            cold_records.is_empty(),
            !case.expected_rule_ids.iter().any(|id| id == rule_id),
            "unexpected result in {}: {cold:#?}",
            case.path,
        );
        assert_eq!(
            cold["raw_signal_quality"]["contract_violations"], 0,
            "finding contract violation in {}: {cold:#?}",
            case.path,
        );
    }
}

fn findings_for_rule<'a>(report: &'a Value, rule_id: &str) -> Vec<&'a Value> {
    report["findings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|finding| finding["rule_id"] == rule_id)
        .collect()
}

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/changed_rules")
}

fn copy_tree(source: &Path, target: &Path) {
    for entry in
        fs::read_dir(source).unwrap_or_else(|error| panic!("read {}: {error}", source.display()))
    {
        let entry = entry.expect("fixture entry");
        let destination = target.join(entry.file_name());
        if entry.path().is_dir() {
            fs::create_dir_all(&destination).expect("create fixture directory");
            copy_tree(&entry.path(), &destination);
        } else {
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent).expect("create fixture parent");
            }
            fs::copy(entry.path(), destination).expect("copy fixture file");
        }
    }
}

fn scan_changed(root: &Path) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_repopilot"))
        .args(["scan", ".", "--changed", "--format", "json"])
        .current_dir(root)
        .output()
        .expect("run changed scan");
    assert!(
        output.status.success(),
        "scan failed: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    serde_json::from_slice(&output.stdout).expect("scan JSON")
}

fn init_repo(root: &Path) {
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "test@example.com"]);
    git(root, &["config", "user.name", "RepoPilot Test"]);
}

fn commit_all(root: &Path, message: &str) {
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", message]);
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("run git");
    assert!(output.status.success(), "git {args:?} failed");
}
