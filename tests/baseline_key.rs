use repopilot::baseline::key::stable_finding_key;
use repopilot::findings::types::{Evidence, Finding, FindingCategory, Severity};
use std::path::{Path, PathBuf};

#[test]
fn same_finding_produces_same_stable_key() {
    let finding = finding("architecture.large-file", "src/main.rs", 1);

    let first = stable_finding_key(&finding, Path::new("."));
    let second = stable_finding_key(&finding, Path::new("."));

    assert_eq!(first, second);
}

#[test]
fn path_separators_are_normalized() {
    let finding = finding("architecture.large-file", "src\\main.rs", 1);

    let key = stable_finding_key(&finding, Path::new("."));

    assert_eq!(key, "architecture.large-file:src/main.rs:1");
}

#[test]
fn absolute_paths_are_not_leaked_into_key() {
    let root = PathBuf::from("/tmp/repopilot-project");
    let finding = finding(
        "security.secret-candidate",
        "/tmp/repopilot-project/src/config.rs",
        42,
    );

    let key = stable_finding_key(&finding, &root);

    assert_eq!(key, "security.secret-candidate:src/config.rs:42");
    assert!(!key.contains("/tmp/repopilot-project"));
}

#[test]
fn key_includes_rule_id_path_and_line() {
    let finding = finding("code-marker.todo", "src/app.rs", 7);

    let key = stable_finding_key(&finding, Path::new("."));

    assert_eq!(key, "code-marker.todo:src/app.rs:7");
}

fn finding(rule_id: &str, path: &str, line: usize) -> Finding {
    Finding {
        id: "generated-id".to_string(),
        rule_id: rule_id.to_string(),
        title: "Finding".to_string(),
        description: "Description".to_string(),
        category: FindingCategory::Architecture,
        severity: Severity::High,
        evidence: vec![Evidence {
            path: PathBuf::from(path),
            line_start: line,
            line_end: None,
            snippet: "snippet".to_string(),
        }],
    }
}
