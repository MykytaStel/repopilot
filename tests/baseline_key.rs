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

    assert!(key.starts_with("architecture.large-file:src/main.rs:"));
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

    assert!(key.starts_with("security.secret-candidate:src/config.rs:"));
    assert!(!key.contains("/tmp/repopilot-project"));
}

#[test]
fn key_includes_rule_id_path_and_stable_identity() {
    let finding = finding("code-marker.todo", "src/app.rs", 7);

    let key = stable_finding_key(&finding, Path::new("."));

    assert!(key.starts_with("code-marker.todo:src/app.rs:"));
    assert!(!key.ends_with(":7"));
}

#[test]
fn key_is_stable_when_only_line_number_shifts() {
    let before = finding("code-marker.todo", "src/app.rs", 7);
    let after = finding("code-marker.todo", "src/app.rs", 12);

    assert_eq!(
        stable_finding_key(&before, Path::new(".")),
        stable_finding_key(&after, Path::new("."))
    );
}

#[test]
fn key_masks_string_literal_value_changes_but_keeps_identifier_changes() {
    let first = finding_with_snippet(
        "security.secret-candidate",
        "src/config.rs",
        1,
        "const API_KEY: &str = \"abc12345\";",
    );
    let rotated_value = finding_with_snippet(
        "security.secret-candidate",
        "src/config.rs",
        8,
        "const API_KEY: &str = \"def67890\";",
    );
    let different_identifier = finding_with_snippet(
        "security.secret-candidate",
        "src/config.rs",
        2,
        "const ACCESS_TOKEN: &str = \"abc12345\";",
    );

    assert_eq!(
        stable_finding_key(&first, Path::new(".")),
        stable_finding_key(&rotated_value, Path::new("."))
    );
    assert_ne!(
        stable_finding_key(&first, Path::new(".")),
        stable_finding_key(&different_identifier, Path::new("."))
    );
}

fn finding(rule_id: &str, path: &str, line: usize) -> Finding {
    finding_with_snippet(rule_id, path, line, "snippet")
}

fn finding_with_snippet(rule_id: &str, path: &str, line: usize, snippet: &str) -> Finding {
    Finding {
        id: "generated-id".to_string(),
        rule_id: rule_id.to_string(),
        recommendation: Finding::recommendation_for_rule_id(rule_id),
        title: "Finding".to_string(),
        description: "Description".to_string(),
        category: FindingCategory::Architecture,
        severity: Severity::High,
        confidence: Default::default(),
        evidence: vec![Evidence {
            path: PathBuf::from(path),
            line_start: line,
            line_end: None,
            snippet: snippet.to_string(),
        }],
        workspace_package: None,
        docs_url: None,
        provenance: Default::default(),
        risk: Default::default(),
    }
}
