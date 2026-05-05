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
        languages: vec![],
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
        }],
    };

    let html = render_scan_summary(&summary, OutputFormat::Html).expect("failed to render html");

    assert!(html.contains("RepoPilot Scan Report"));
    assert!(html.contains("<div class=\"num\">1</div><div class=\"label\">Files</div>"));
    assert!(html.contains("security.secret-candidate"));
    assert!(html.contains("API_KEY = &quot;abc&lt;123&gt;&quot;"));
    assert!(!html.contains("API_KEY = \"abc<123>\""));
}
