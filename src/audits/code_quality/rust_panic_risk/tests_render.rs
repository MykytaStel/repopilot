//! Report-renderer carve-out tests: infallible `write!`/`writeln!` result
//! unwraps in renderers are ignored, while genuine unwraps in the same modules
//! still report.

use super::*;
use crate::scan::facts::FileFacts;
use std::path::PathBuf;

#[test]
fn ignores_string_write_unwraps_in_report_renderers() {
    let file = facts(
        "src/output/markdown.rs",
        "pub fn render(output: &mut String) {\n    writeln!(output, \"# Report\").unwrap();\n}\n",
        false,
    );

    let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());

    assert!(findings.is_empty());
}

#[test]
fn ignores_multiline_string_write_unwraps_in_report_renderers() {
    let file = facts(
        "src/output/console.rs",
        "pub fn render(output: &mut String) {\n    writeln!(\n        output,\n        \"Findings: {}\",\n        3\n    )\n    .unwrap();\n}\n",
        false,
    );

    let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());

    assert!(findings.is_empty());
}

#[test]
fn still_reports_non_renderer_unwraps_in_output_modules() {
    let file = facts(
        "src/output/markdown.rs",
        "pub fn render(value: Option<&str>) -> &str {\n    value.unwrap()\n}\n",
        false,
    );

    let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, RULE_ID);
}

#[test]
fn still_reports_unwraps_inside_renderer_format_arguments() {
    let file = facts(
        "src/output/console.rs",
        "pub fn render(output: &mut String, value: Option<&str>) {\n    writeln!(output, \"{}\", value.unwrap());\n}\n",
        false,
    );

    let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, RULE_ID);
}

fn facts(path: &str, content: &str, has_inline_tests: bool) -> FileFacts {
    FileFacts {
        path: PathBuf::from(path),
        language: Some("Rust".to_string()),
        non_empty_lines: content.lines().count(),
        branch_count: 0,
        imports: Vec::new(),
        content: Some(content.to_string()),
        has_inline_tests,
        in_executable_package: false,
        deferred_imports: Vec::new(),
    }
}
