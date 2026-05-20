use repopilot::findings::types::{Evidence, Finding, FindingCategory, Severity};
use repopilot::output::{OutputFormat, render_scan_summary};
use repopilot::scan::types::{HiddenSuggestionSummary, ScanSummary};
use std::path::PathBuf;

#[test]
fn html_output_escapes_snippets_and_renders_summary() {
    let summary = ScanSummary {
        root_path: PathBuf::from("demo"),
        mode: Default::default(),
        base_ref: None,
        changed_files_count: 0,
        repo_level_rules_included: true,
        files_discovered: 0,
        files_analyzed: 1,
        directories_count: 1,
        non_empty_lines: 3,
        large_files_skipped: 0,
        files_skipped_low_signal: 0,
        binary_files_skipped: 0,
        skipped_bytes: 0,
        languages: vec![],
        detected_frameworks: vec![],
        framework_projects: vec![],
        findings: vec![Finding {
            id: "finding-1".to_string(),
            rule_id: "security.secret-candidate".to_string(),
            recommendation: Finding::recommendation_for_rule_id("security.secret-candidate"),
            title: "Possible secret detected".to_string(),
            description: "description".to_string(),
            category: FindingCategory::Security,
            severity: Severity::High,
            confidence: Default::default(),
            evidence: vec![Evidence {
                path: PathBuf::from("src/config.rs"),
                line_start: 1,
                line_end: None,
                snippet: "API_KEY = \"abc<123>\"".to_string(),
            }],
            workspace_package: None,
            docs_url: None,
            provenance: Default::default(),
            risk: Default::default(),
        }],
        react_native: None,
        coupling_graph: None,
        scan_duration_us: 0,
        health_score: 0,
        visible_findings_count: 1,
        hidden_suggestions_count: 0,
        hidden_suggestions: Vec::new(),
        visibility_profile: None,
        files_skipped_by_limit: 0,
        files_skipped_repopilotignore: 0,
        repopilotignore_path: None,
        scan_timings: None,
        cache_telemetry: None,
        local_feedback: None,
        diagnostics: Vec::new(),
    };

    let html = render_scan_summary(&summary, OutputFormat::Html).expect("failed to render html");

    assert!(html.contains("RepoPilot Scan Report"));
    assert!(html.contains(&format!(
        "RepoPilot version: <strong>{}</strong>",
        env!("CARGO_PKG_VERSION")
    )));
    assert!(html.contains("<div class=\"label\">Risk</div>"));
    assert!(html.contains("<h2>Risk Summary</h2>"));
    assert!(html.contains("<h2>Top Rules</h2>"));
    assert!(html.contains("data-filter-type=\"severity\""));
    assert!(html.contains("data-filter-type=\"category\""));
    assert!(html.contains("data-filter-type=\"rule\""));
    assert!(html.contains("class=\"finding-group\""));
    assert!(html.contains("class=\"finding-card\""));
    assert!(html.contains("security.secret-candidate"));
    assert!(html.contains("<strong>Context:</strong> description"));
    assert!(html.contains("<strong>Recommendation:</strong>"));
    assert!(html.contains("API_KEY = &quot;abc&lt;123&gt;&quot;"));
    assert!(!html.contains("API_KEY = \"abc<123>\""));
}

#[test]
fn html_output_includes_hidden_suggestion_breakdown() {
    let summary = ScanSummary {
        root_path: PathBuf::from("demo"),
        hidden_suggestions_count: 1,
        hidden_suggestions: vec![HiddenSuggestionSummary {
            intent: "runtime-risk".to_string(),
            rule_id: "language.javascript.runtime-exit-risk".to_string(),
            category: "code-quality".to_string(),
            reason: "process exit in script/tooling boundary is a strict-mode suggestion"
                .to_string(),
            count: 1,
        }],
        ..ScanSummary::default()
    };

    let html = render_scan_summary(&summary, OutputFormat::Html).expect("failed to render html");

    assert!(html.contains("Top Hidden Suggestions"));
    assert!(html.contains("language.javascript.runtime-exit-risk"));
    assert!(html.contains("code-quality"));
}
