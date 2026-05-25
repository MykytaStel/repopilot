use repopilot::findings::quality::summarize_signal_quality;
use repopilot::findings::types::{Confidence, Evidence, Finding, FindingCategory, Severity};
use repopilot::output::{
    ColorChoice, ColorDestination, ConsoleOutputStyle, FindingRenderLimit, OutputFormat,
    RenderOptions, render_scan_summary_with_options,
};
use repopilot::risk::{RiskInputs, assess_finding};
use repopilot::scan::types::{HiddenSuggestionSummary, LanguageSummary, ScanSummary};
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn golden_console_compact_output_stays_stable() {
    let output = render(OutputFormat::Console, ConsoleOutputStyle::Compact);

    assert_golden(
        "scan-console-compact.txt",
        output,
        include_str!("fixtures/golden/scan-console-compact.txt"),
    );
}

#[test]
fn golden_console_full_output_stays_stable() {
    let output = render(OutputFormat::Console, ConsoleOutputStyle::Full);

    assert_golden(
        "scan-console-full.txt",
        output,
        include_str!("fixtures/golden/scan-console-full.txt"),
    );
}

#[test]
fn golden_markdown_output_stays_stable() {
    let output = render(OutputFormat::Markdown, ConsoleOutputStyle::Full);

    assert_golden(
        "scan-markdown.md",
        output,
        include_str!("fixtures/golden/scan-markdown.md"),
    );
}

#[test]
fn golden_json_schema_output_stays_stable() {
    let output = render(OutputFormat::Json, ConsoleOutputStyle::Full);

    assert_golden(
        "scan-json-schema.golden.json",
        output,
        include_str!("fixtures/golden/scan-json-schema.golden.json"),
    );
}

#[test]
fn golden_sarif_output_stays_stable() {
    let output = render(OutputFormat::Sarif, ConsoleOutputStyle::Full);

    assert_golden(
        "scan-sarif.golden.json",
        output,
        include_str!("fixtures/golden/scan-sarif.golden.json"),
    );
}

fn render(format: OutputFormat, console_output_style: ConsoleOutputStyle) -> String {
    render_scan_summary_with_options(
        &golden_summary(),
        format,
        RenderOptions {
            console_output_style,
            color_choice: ColorChoice::Never,
            color_destination: ColorDestination::Stdout,
            quiet: false,
            findings_limit: FindingRenderLimit::Default,
        },
    )
    .expect("golden output should render")
}

fn golden_summary() -> ScanSummary {
    let findings = vec![secret_finding()];
    let signal_quality = summarize_signal_quality(&findings);

    ScanSummary {
        root_path: PathBuf::from("demo-project"),
        files_discovered: 4,
        files_analyzed: 2,
        directories_count: 2,
        non_empty_lines: 42,
        languages: vec![
            LanguageSummary {
                name: "Rust".to_string(),
                files_analyzed: 1,
            },
            LanguageSummary {
                name: "TypeScript".to_string(),
                files_analyzed: 1,
            },
        ],
        findings,
        health_score: 90,
        raw_findings_count: 1,
        visible_findings_count: 1,
        hidden_suggestions_count: 2,
        hidden_suggestions: vec![HiddenSuggestionSummary {
            intent: "maintainability".to_string(),
            rule_id: "code-quality.long-function".to_string(),
            category: "code-quality".to_string(),
            reason: "maintainability signals are hidden in the default profile".to_string(),
            count: 2,
        }],
        visibility_profile: Some("default".to_string()),
        raw_signal_quality: signal_quality.clone(),
        visible_signal_quality: signal_quality.clone(),
        signal_quality,
        ..ScanSummary::default()
    }
}

fn secret_finding() -> Finding {
    let mut finding = Finding {
        id: "security.secret-candidate:src/config.ts:3".to_string(),
        rule_id: "security.secret-candidate".to_string(),
        title: String::new(),
        description: String::new(),
        recommendation: String::new(),
        category: FindingCategory::Security,
        severity: Severity::High,
        confidence: Confidence::High,
        evidence: vec![Evidence {
            path: PathBuf::from("src/config.ts"),
            line_start: 3,
            line_end: None,
            snippet: r#"export const API_TOKEN = "<redacted>";"#.to_string(),
        }],
        workspace_package: None,
        docs_url: None,
        provenance: Default::default(),
        risk: Default::default(),
    };
    finding.populate_rule_metadata();
    finding.risk = assess_finding(&finding, None, RiskInputs::default());
    finding
}

fn assert_golden(name: &str, actual: String, expected: &str) {
    let actual = normalize_snapshot(actual);

    if std::env::var_os("REPOPILOT_UPDATE_GOLDEN").is_some() {
        fs::write(golden_path(name), actual).expect("failed to update golden fixture");
        return;
    }

    assert_eq!(actual, expected, "golden fixture changed: {name}");
}

fn normalize_snapshot(output: String) -> String {
    let mut output = output.replace("\r\n", "\n");
    output = output.replace(env!("CARGO_PKG_VERSION"), "{{VERSION}}");
    if !output.ends_with('\n') {
        output.push('\n');
    }
    output
}

fn golden_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("golden")
        .join(name)
}
