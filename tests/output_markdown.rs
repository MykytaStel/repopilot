use repopilot::findings::types::{Evidence, Finding, FindingCategory, Severity};
use repopilot::output::{OutputFormat, render_scan_summary};
use repopilot::scan::types::{LanguageSummary, ScanSummary};
use std::path::PathBuf;

#[test]
fn renders_markdown_scan_summary() {
    let summary = ScanSummary {
        root_path: PathBuf::from("demo-project"),
        files_count: 2,
        directories_count: 1,
        lines_of_code: 10,
        languages: vec![
            LanguageSummary {
                name: "Rust".to_string(),
                files_count: 1,
            },
            LanguageSummary {
                name: "TypeScript".to_string(),
                files_count: 1,
            },
        ],
        findings: vec![Finding {
            id: "code-marker.todo.src/main.rs:7".to_string(),
            rule_id: "code-marker.todo".to_string(),
            title: "TODO marker found".to_string(),
            description: "A TODO marker was found in the codebase and should be reviewed."
                .to_string(),
            category: FindingCategory::CodeQuality,
            severity: Severity::Low,
            evidence: vec![Evidence {
                path: PathBuf::from("src/main.rs"),
                line_start: 7,
                line_end: None,
                snippet: "// TODO: improve architecture".to_string(),
            }],
        }],
    };

    let output = render_scan_summary(&summary, OutputFormat::Markdown)
        .expect("failed to render markdown summary");

    assert!(output.contains("# RepoPilot Scan Report"));
    assert!(output.contains("## Summary"));
    assert!(output.contains("## Languages"));
    assert!(output.contains("## Findings"));
    assert!(output.contains("| LOW | `code-marker.todo` | TODO marker found |"));
    assert!(output.contains("`src/main.rs:7`"));
    assert!(output.contains("- **Path:** `demo-project`"));
    assert!(output.contains("- **Files analyzed:** 2"));
    assert!(output.contains("| Rust | 1 |"));
    assert!(output.contains("| TypeScript | 1 |"));
    assert!(output.contains("| TODO | `src/main.rs` | 7 | // TODO: improve architecture |"));
}

#[test]
fn renders_empty_markdown_sections() {
    let summary = ScanSummary {
        root_path: PathBuf::from("empty-project"),
        files_count: 0,
        directories_count: 0,
        lines_of_code: 0,
        languages: vec![],
        findings: vec![],
    };

    let output = render_scan_summary(&summary, OutputFormat::Markdown)
        .expect("failed to render markdown summary");

    assert!(output.contains("No languages detected."));
    assert!(output.contains("No findings found."));
    assert!(output.contains("No TODO/FIXME/HACK markers found."));
}
