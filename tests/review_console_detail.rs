use repopilot::output::{DetailLevel, FindingRenderLimit, OutputFormat};
use repopilot::review::diff::{ChangeStatus, ChangedFile};
use repopilot::review::model::ReviewReport;
use repopilot::review::render::{ReviewRenderOptions, render_with_options};
use repopilot::scan::types::{ScanMetadata, ScanSummary};
use std::path::PathBuf;

#[test]
fn findings_detail_bounds_changed_files_and_summarizes_areas() {
    let report = report_with_changed_files();

    let output = render_console(&report, DetailLevel::Findings);

    assert!(output.contains("Changed areas:\n"));
    assert!(output.contains("  docs: 2\n"));
    assert!(output.contains("  root: 1\n"));
    assert!(output.contains("  src: 8\n"));
    assert!(output.contains("  tests: 3\n"));
    assert!(output.contains("  tools: 1\n"));
    assert!(output.contains("Modified src/file-0.rs"));
    assert!(output.contains("Modified docs/page-0.md"));
    assert!(!output.contains("Modified docs/page-1.md"));
    assert!(output.contains("... 3 more changed file(s); rerun with --detail full"));
    assert!(output.contains("Next:\n"));
    assert!(output.contains("Rerun with --detail full"));
}

#[test]
fn full_detail_keeps_every_changed_file_and_summary_keeps_none() {
    let report = report_with_changed_files();

    let full = render_console(&report, DetailLevel::Full);
    let summary = render_console(&report, DetailLevel::Summary);

    assert!(full.contains("Modified tools/check.rs"));
    assert!(full.contains("Modified docs/page-1.md"));
    assert!(!full.contains("more changed file(s)"));
    assert!(!summary.contains("\nChanged files:\n"));
    assert!(!summary.contains("Changed areas:\n"));
}

#[test]
fn machine_and_markdown_inventories_remain_complete() {
    let report = report_with_changed_files();
    let options = options(DetailLevel::Findings);

    let json = render_with_options(&report, OutputFormat::Json, None, None, options)
        .expect("JSON review should render");
    let markdown = render_with_options(&report, OutputFormat::Markdown, None, None, options)
        .expect("Markdown review should render");
    let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON review");

    assert_eq!(value["changed_files"].as_array().map(Vec::len), Some(15));
    assert!(markdown.contains("`docs/page-1.md`"));
}

fn render_console(report: &ReviewReport, detail: DetailLevel) -> String {
    render_with_options(report, OutputFormat::Console, None, None, options(detail))
        .expect("console review should render")
}

fn options(detail: DetailLevel) -> ReviewRenderOptions {
    ReviewRenderOptions {
        detail,
        findings_limit: FindingRenderLimit::Default,
    }
}

fn report_with_changed_files() -> ReviewReport {
    let paths = (0..8)
        .map(|index| format!("src/file-{index}.rs"))
        .chain((0..3).map(|index| format!("tests/test-{index}.rs")))
        .chain((0..2).map(|index| format!("docs/page-{index}.md")))
        .chain(["README.md".to_string(), "tools/check.rs".to_string()]);

    ReviewReport {
        summary: ScanSummary {
            metadata: ScanMetadata {
                root_path: PathBuf::from("."),
                ..Default::default()
            },
            ..Default::default()
        },
        repo_root: PathBuf::from("."),
        baseline_path: None,
        changed_files: paths
            .map(|path| ChangedFile {
                path: PathBuf::from(path),
                status: ChangeStatus::Modified,
                ranges: Vec::new(),
                hunks: Vec::new(),
            })
            .collect(),
        blast_radius: Vec::new(),
        impact_paths: Default::default(),
        ownership: Default::default(),
        ownership_diagnostics: Vec::new(),
        boundary_signals: Vec::new(),
        boundary_missing_test: false,
        tiered_signals: Default::default(),
        timings: Default::default(),
        verification: Vec::new(),
        findings: Vec::new(),
    }
}
