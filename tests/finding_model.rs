use repopilot::findings::types::{Confidence, Evidence, Finding, FindingCategory, Severity};
use std::path::PathBuf;

#[test]
fn finding_contains_evidence() {
    let finding = Finding {
        id: "code-marker.todo.src/main.rs:10".to_string(),
        rule_id: "code-marker.todo".to_string(),
        recommendation: Finding::recommendation_for_rule_id("code-marker.todo"),
        title: "TODO marker found".to_string(),
        description: "A TODO marker was found.".to_string(),
        category: FindingCategory::CodeQuality,
        severity: Severity::Low,
        confidence: Default::default(),
        evidence: vec![Evidence {
            path: PathBuf::from("src/main.rs"),
            line_start: 10,
            line_end: None,
            snippet: "// TODO: improve this".to_string(),
        }],
        workspace_package: None,
        docs_url: None,
        provenance: Default::default(),
        risk: Default::default(),
    };

    assert_eq!(finding.rule_id, "code-marker.todo");
    assert!(!finding.recommendation.is_empty());
    assert_eq!(finding.category, FindingCategory::CodeQuality);
    assert_eq!(finding.confidence, Confidence::Medium);
    assert_eq!(finding.severity_label(), "LOW");
    assert_eq!(finding.confidence_label(), "MEDIUM");
    assert_eq!(finding.evidence.len(), 1);
    assert_eq!(finding.evidence[0].line_start, 10);
}

#[test]
fn missing_confidence_deserializes_as_medium() {
    let finding: Finding = serde_json::from_value(serde_json::json!({
        "id": "code-marker.todo.src/main.rs:10",
        "rule_id": "code-marker.todo",
        "title": "TODO marker found",
        "description": "A TODO marker was found.",
        "category": "CODE_QUALITY",
        "severity": "LOW",
        "evidence": [{
            "path": "src/main.rs",
            "line_start": 10,
            "line_end": null,
            "snippet": "// TODO: improve this"
        }]
    }))
    .expect("old finding JSON without confidence should deserialize");

    assert_eq!(finding.confidence, Confidence::Medium);
}

#[test]
fn missing_recommendation_deserializes_as_empty_for_old_reports() {
    let finding: Finding = serde_json::from_value(serde_json::json!({
        "id": "code-marker.todo.src/main.rs:10",
        "rule_id": "code-marker.todo",
        "title": "TODO marker found",
        "description": "A TODO marker was found.",
        "category": "CODE_QUALITY",
        "severity": "LOW",
        "confidence": "MEDIUM",
        "evidence": [{
            "path": "src/main.rs",
            "line_start": 10,
            "line_end": null,
            "snippet": "// TODO: improve this"
        }]
    }))
    .expect("old finding JSON without recommendation should deserialize");

    assert!(finding.recommendation.is_empty());
}
