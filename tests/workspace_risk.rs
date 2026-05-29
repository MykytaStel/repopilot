use repopilot::findings::types::{Evidence, Finding, FindingCategory, Severity};
use repopilot::output::{OutputFormat, render_scan_summary};
use repopilot::scan::types::{ScanArtifacts, ScanMetadata, ScanSummary};
use std::path::PathBuf;

fn make_finding(pkg: Option<&str>, severity: Severity) -> Finding {
    Finding {
        id: String::new(),
        rule_id: "security.secret-candidate".to_owned(),
        recommendation: Finding::recommendation_for_rule_id("security.secret-candidate"),
        title: "Test".to_owned(),
        description: "desc".to_owned(),
        category: FindingCategory::Security,
        severity,
        confidence: Default::default(),
        evidence: vec![Evidence {
            path: PathBuf::from("src/main.rs"),
            line_start: 1,
            line_end: None,
            snippet: String::new(),
        }],
        workspace_package: pkg.map(str::to_owned),
        docs_url: None,
        provenance: Default::default(),
        risk: Default::default(),
    }
}

#[test]
fn workspace_risk_not_rendered_when_no_workspace_package() {
    let summary = ScanSummary {
        metadata: ScanMetadata {
            root_path: PathBuf::from("."),
            ..Default::default()
        },
        artifacts: ScanArtifacts {
            findings: vec![
                make_finding(None, Severity::High),
                make_finding(None, Severity::Medium),
            ],
            ..Default::default()
        },
        ..ScanSummary::default()
    };

    let console = render_scan_summary(&summary, OutputFormat::Console).unwrap();
    assert!(
        !console.contains("Workspace Risk"),
        "workspace risk table must not appear when no finding has a workspace_package"
    );

    let markdown = render_scan_summary(&summary, OutputFormat::Markdown).unwrap();
    assert!(
        !markdown.contains("Workspace Risk"),
        "workspace risk section must not appear in markdown when no workspace_package is set"
    );
}

#[test]
fn workspace_risk_table_aggregates_correctly() {
    let summary = ScanSummary {
        metadata: ScanMetadata {
            root_path: PathBuf::from("."),
            ..Default::default()
        },
        artifacts: ScanArtifacts {
            findings: vec![
                make_finding(Some("web"), Severity::Critical),
                make_finding(Some("web"), Severity::High),
                make_finding(Some("api"), Severity::Medium),
            ],
            ..Default::default()
        },
        ..ScanSummary::default()
    };

    let console = render_scan_summary(&summary, OutputFormat::Console).unwrap();
    assert!(console.contains("Workspace Risk"), "risk table must appear");
    // web has 2 findings, api has 1
    assert!(console.contains("web"), "web package must appear");
    assert!(console.contains("api"), "api package must appear");
}

#[test]
fn workspace_risk_markdown_contains_table_header() {
    let summary = ScanSummary {
        metadata: ScanMetadata {
            root_path: PathBuf::from("."),
            ..Default::default()
        },
        artifacts: ScanArtifacts {
            findings: vec![make_finding(Some("core"), Severity::High)],
            ..Default::default()
        },
        ..ScanSummary::default()
    };

    let markdown = render_scan_summary(&summary, OutputFormat::Markdown).unwrap();
    assert!(
        markdown.contains("## Workspace Risk Summary"),
        "markdown must include the Workspace Risk Summary heading"
    );
    assert!(
        markdown.contains("| Package |"),
        "markdown table must include Package column header"
    );
    assert!(
        markdown.contains("core"),
        "package name must appear in the table"
    );
}
