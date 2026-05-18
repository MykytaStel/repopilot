use repopilot::compare::diff::diff_summaries;
use repopilot::compare::render::render_json;
use repopilot::findings::types::{Evidence, Finding, FindingCategory, Severity};
use repopilot::scan::types::ScanSummary;
use std::path::PathBuf;

#[test]
fn compare_uses_rule_and_evidence_as_stable_key() {
    let before = summary(vec![finding(
        "old-generated-id",
        "architecture.large-file",
        "src/main.rs",
        1,
        Severity::Medium,
    )]);
    let after = summary(vec![finding(
        "new-generated-id",
        "architecture.large-file",
        "src/main.rs",
        1,
        Severity::High,
    )]);

    let diff = diff_summaries(&before, &after);

    assert!(diff.new_findings.is_empty());
    assert!(diff.resolved_findings.is_empty());
    assert_eq!(diff.severity_increased.len(), 1);
}

#[test]
fn compare_reports_new_and_resolved_findings() {
    let before = summary(vec![finding(
        "a",
        "code-marker.todo",
        "src/old.rs",
        2,
        Severity::Low,
    )]);
    let after = summary(vec![finding(
        "b",
        "security.secret-candidate",
        "src/new.rs",
        4,
        Severity::High,
    )]);

    let diff = diff_summaries(&before, &after);

    assert_eq!(diff.new_findings.len(), 1);
    assert_eq!(diff.resolved_findings.len(), 1);
}

#[test]
fn compare_json_renders_valid_json() {
    let diff = diff_summaries(&summary(vec![]), &summary(vec![]));

    let rendered = render_json(&diff).expect("failed to render json");
    let value: serde_json::Value = serde_json::from_str(&rendered).expect("invalid json");

    assert_eq!(value["new_findings"].as_array().unwrap().len(), 0);
}

fn summary(findings: Vec<Finding>) -> ScanSummary {
    let visible_findings_count = findings.len();
    ScanSummary {
        root_path: PathBuf::from("demo"),
        mode: Default::default(),
        base_ref: None,
        changed_files_count: 0,
        repo_level_rules_included: true,
        files_discovered: 0,
        files_analyzed: 1,
        directories_count: 0,
        non_empty_lines: 10,
        large_files_skipped: 0,
        files_skipped_low_signal: 0,
        binary_files_skipped: 0,
        skipped_bytes: 0,
        languages: vec![],
        findings,
        detected_frameworks: vec![],
        framework_projects: vec![],
        react_native: None,
        coupling_graph: None,
        scan_duration_us: 0,
        health_score: 0,
        visible_findings_count,
        hidden_suggestions_count: 0,
        hidden_suggestions: Vec::new(),
        visibility_profile: None,
        files_skipped_by_limit: 0,
        files_skipped_repopilotignore: 0,
        repopilotignore_path: None,
        scan_timings: None,
        cache_telemetry: None,
    }
}

fn finding(id: &str, rule_id: &str, path: &str, line: usize, severity: Severity) -> Finding {
    Finding {
        id: id.to_string(),
        rule_id: rule_id.to_string(),
        recommendation: Finding::recommendation_for_rule_id(rule_id),
        title: "Finding".to_string(),
        description: "Description".to_string(),
        category: FindingCategory::Architecture,
        severity,
        confidence: Default::default(),
        evidence: vec![Evidence {
            path: PathBuf::from(path),
            line_start: line,
            line_end: None,
            snippet: "snippet".to_string(),
        }],
        workspace_package: None,
        docs_url: None,
        risk: Default::default(),
    }
}
