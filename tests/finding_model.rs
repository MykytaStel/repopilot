use repopilot::findings::types::{Evidence, Finding, FindingCategory, Severity};
use std::path::PathBuf;

#[test]
fn finding_contains_evidence() {
    let finding = Finding {
        id: "code-marker.todo.src/main.rs:10".to_string(),
        rule_id: "code-marker.todo".to_string(),
        title: "TODO marker found".to_string(),
        description: "A TODO marker was found.".to_string(),
        category: FindingCategory::CodeQuality,
        severity: Severity::Low,
        evidence: vec![Evidence {
            path: PathBuf::from("src/main.rs"),
            line_start: 10,
            line_end: None,
            snippet: "// TODO: improve this".to_string(),
        }],
    };

    assert_eq!(finding.rule_id, "code-marker.todo");
    assert_eq!(finding.category, FindingCategory::CodeQuality);
    assert_eq!(finding.severity, Severity::Low);
    assert_eq!(finding.severity_label(), "LOW");
    assert_eq!(finding.evidence.len(), 1);
    assert_eq!(finding.evidence[0].line_start, 10);
}
