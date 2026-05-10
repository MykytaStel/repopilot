use repopilot::findings::types::{Evidence, Finding, FindingCategory, Severity};
use repopilot::output::{OutputFormat, render_scan_summary};
use repopilot::scan::types::ScanSummary;
use std::path::PathBuf;

#[test]
fn html_output_escapes_snippets_and_renders_summary() {
    let summary = ScanSummary {
        root_path: PathBuf::from("demo"),
        files_count: 1,
        directories_count: 1,
        lines_of_code: 3,
        skipped_files_count: 0,
        skipped_bytes: 0,
        languages: vec![],
        detected_frameworks: vec![],
        framework_projects: vec![],
        findings: vec![Finding {
            id: "finding-1".to_string(),
            rule_id: "security.secret-candidate".to_string(),
            title: "Possible secret detected".to_string(),
            description: "description".to_string(),
            category: FindingCategory::Security,
            severity: Severity::High,
            evidence: vec![Evidence {
                path: PathBuf::from("src/config.rs"),
                line_start: 1,
                line_end: None,
                snippet: "API_KEY = \"abc<123>\"".to_string(),
            }],
            workspace_package: None,
            docs_url: None,
        }],
        react_native: None,
        coupling_graph: None,
        scan_duration_us: 0,
        health_score: 0,
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
    assert!(html.contains("API_KEY = &quot;abc&lt;123&gt;&quot;"));
    assert!(!html.contains("API_KEY = \"abc<123>\""));
}
