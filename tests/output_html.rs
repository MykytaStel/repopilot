use repopilot::baseline::diff::{BaselineScanReport, BaselineStatus, FindingBaselineStatus};
use repopilot::findings::types::{Evidence, Finding, FindingCategory, Severity};
use repopilot::output::{OutputFormat, render_baseline_scan_report, render_scan_summary};
use repopilot::scan::types::{
    HiddenSuggestionSummary, ScanArtifacts, ScanMetadata, ScanMetrics, ScanSummary,
};
use std::path::PathBuf;

#[test]
fn html_output_escapes_snippets_and_renders_summary() {
    let summary = ScanSummary {
        metadata: ScanMetadata {
            root_path: PathBuf::from("demo"),
            ..Default::default()
        },
        metrics: ScanMetrics {
            files_analyzed: 1,
            directories_count: 1,
            non_empty_lines: 3,
            raw_findings_count: 1,
            visible_findings_count: 1,
            ..Default::default()
        },
        artifacts: ScanArtifacts {
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
            ..Default::default()
        },
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
        metadata: ScanMetadata {
            root_path: PathBuf::from("demo"),
            ..Default::default()
        },
        metrics: ScanMetrics {
            hidden_suggestions_count: 1,
            ..Default::default()
        },
        artifacts: ScanArtifacts {
            hidden_suggestions: vec![HiddenSuggestionSummary {
                intent: "runtime-risk".to_string(),
                rule_id: "language.javascript.runtime-exit-risk".to_string(),
                category: "code-quality".to_string(),
                reason: "process exit in script/tooling boundary is a strict-mode suggestion"
                    .to_string(),
                count: 1,
            }],
            ..Default::default()
        },
    };

    let html = render_scan_summary(&summary, OutputFormat::Html).expect("failed to render html");

    assert!(html.contains("Top Hidden Suggestions"));
    assert!(html.contains("language.javascript.runtime-exit-risk"));
    assert!(html.contains("code-quality"));
}

#[test]
fn html_baseline_status_stays_aligned_for_duplicate_findings() {
    let report = duplicate_status_report();

    let html = render_baseline_scan_report(&report, OutputFormat::Html, None)
        .expect("failed to render baseline html");

    let existing = html
        .find("baseline: existing")
        .expect("existing status should render");
    let new = html
        .find("baseline: new")
        .expect("new status should render");
    assert!(existing < new);
}

fn duplicate_status_report() -> BaselineScanReport {
    BaselineScanReport {
        summary: ScanSummary {
            metadata: ScanMetadata {
                root_path: PathBuf::from("demo"),
                ..Default::default()
            },
            artifacts: ScanArtifacts {
                findings: vec![
                    duplicate_finding("existing", 7),
                    duplicate_finding("new", 8),
                ],
                ..Default::default()
            },
            ..Default::default()
        },
        baseline_path: None,
        findings: vec![
            FindingBaselineStatus {
                key: "existing".to_string(),
                status: BaselineStatus::Existing,
            },
            FindingBaselineStatus {
                key: "new".to_string(),
                status: BaselineStatus::New,
            },
        ],
    }
}

fn duplicate_finding(id: &str, line: usize) -> Finding {
    Finding {
        id: id.to_string(),
        rule_id: "code-quality.long-function".to_string(),
        recommendation: Finding::recommendation_for_rule_id("code-quality.long-function"),
        title: "Duplicate-looking finding".to_string(),
        description: "Duplicate-looking finding for renderer status alignment.".to_string(),
        category: FindingCategory::CodeQuality,
        severity: Severity::Medium,
        confidence: Default::default(),
        evidence: vec![Evidence {
            path: PathBuf::from("src/lib.rs"),
            line_start: line,
            line_end: None,
            snippet: "fn duplicate() {}".to_string(),
        }],
        workspace_package: None,
        docs_url: None,
        provenance: Default::default(),
        risk: Default::default(),
    }
}
