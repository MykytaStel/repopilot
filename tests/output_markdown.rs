
use repopilot::output::{OutputFormat, render_scan_summary};
use repopilot::scan::types::{CodeMarker, LanguageSummary, MarkerKind, ScanSummary};
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
        markers: vec![CodeMarker {
            kind: MarkerKind::Todo,
            path: PathBuf::from("src/main.rs"),
            line_number: 7,
            text: "// TODO: improve architecture".to_string(),
        }],
    };

    let output = render_scan_summary(&summary, OutputFormat::Markdown)
        .expect("failed to render markdown summary");

    assert!(output.contains("# RepoPilot Scan Report"));
    assert!(output.contains("## Summary"));
    assert!(output.contains("## Languages"));
    assert!(output.contains("## Code Markers"));

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
        markers: vec![],
    };

    let output = render_scan_summary(&summary, OutputFormat::Markdown)
        .expect("failed to render markdown summary");

    assert!(output.contains("No languages detected."));
    assert!(output.contains("No TODO/FIXME/HACK markers found."));
}