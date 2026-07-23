use super::*;
use crate::findings::types::{Evidence, Finding};

#[test]
fn parses_minimal_feedback_suppressions() {
    let suppressions = parse_suppressions(
        r#"
suppressions:
  - rule_id: architecture.large-file
    path: src/generated/schema.rs
    reason: generated schema boundary
"#,
    );

    assert_eq!(suppressions.len(), 1);
    assert_eq!(suppressions[0].index, 1);
    assert_eq!(suppressions[0].rule_id, "architecture.large-file");
    assert_eq!(suppressions[0].path, "src/generated/schema.rs");
    assert_eq!(
        suppressions[0].reason.as_deref(),
        Some("generated schema boundary")
    );
}

#[test]
fn reports_malformed_yaml_as_warning() {
    let validation = validate_feedback_content(
        "suppressions:\n  - rule_id: [\n",
        PathBuf::from(".repopilot/feedback.yml"),
    );

    assert!(validation.parse_error.is_some());
    assert_eq!(validation.diagnostics.len(), 1);
    assert_eq!(validation.diagnostics[0].code, "feedback.parse-failed");
}

#[test]
fn rejects_incomplete_suppressions() {
    let validation = validate_feedback_content(
        r#"
suppressions:
  - rule_id: security.secret-candidate
  - path: src/main.rs
"#,
        PathBuf::from(".repopilot/feedback.yml"),
    );

    assert_eq!(validation.suppressions.len(), 0);
    assert_eq!(validation.invalid_suppressions_count, 2);
    assert_eq!(validation.diagnostics.len(), 3);
}

#[test]
fn indexed_matching_considers_all_evidence_paths_and_suppression_order() {
    let suppressions = vec![
        LocalSuppression {
            index: 1,
            rule_id: "security.secret-candidate".to_string(),
            path: "src/second.rs".to_string(),
            reason: None,
        },
        LocalSuppression {
            index: 2,
            rule_id: "security.secret-candidate".to_string(),
            path: "src/first.rs".to_string(),
            reason: None,
        },
        LocalSuppression {
            index: 3,
            rule_id: "architecture.large-file".to_string(),
            path: "src/first.rs".to_string(),
            reason: None,
        },
    ];
    let index = build_suppression_index(&suppressions);
    let finding = Finding {
        rule_id: "security.secret-candidate".to_string(),
        evidence: vec![
            Evidence {
                path: PathBuf::from("./src/first.rs"),
                line_start: 1,
                line_end: None,
                snippet: String::new(),
            },
            Evidence {
                path: PathBuf::from("src/second.rs"),
                line_start: 2,
                line_end: None,
                snippet: String::new(),
            },
        ],
        ..Finding::default()
    };

    assert_eq!(matching_suppression_index(&finding, &index), Some(1));
}

#[test]
fn existing_feedback_file_gets_a_deprecation_diagnostic() {
    let content = r#"
suppressions:
  - rule_id: architecture.large-file
    path: src/big.rs
"#;
    let validation = validate_feedback_content(content, PathBuf::from(".repopilot/feedback.yml"));

    assert!(
        validation
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "feedback.deprecated"),
        "expected a feedback.deprecated diagnostic, got: {:?}",
        validation.diagnostics
    );
}
