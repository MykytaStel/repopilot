use repopilot::audits::code_quality::code_markers::detect_code_marker_findings;
use repopilot::findings::types::{FindingCategory, Severity};
use repopilot::scan::facts::FileFacts;
use std::path::{Path, PathBuf};

#[test]
fn converts_code_markers_into_evidence_backed_findings() {
    let content = "\
fn main() {}

// TODO: improve scanner
// FIXME: handle edge case
// HACK: temporary workaround
";

    let file = FileFacts {
        path: PathBuf::from("src/main.rs"),
        language: Some("Rust".to_string()),
        lines_of_code: 5,
        content: content.to_string(),
    };

    let findings = detect_code_marker_findings(&file);

    assert_eq!(findings.len(), 3);

    assert_eq!(findings[0].rule_id, "code-marker.todo");
    assert_eq!(findings[0].category, FindingCategory::CodeQuality);
    assert_eq!(findings[0].severity, Severity::Low);
    assert_eq!(findings[0].evidence[0].path, Path::new("src/main.rs"));
    assert_eq!(findings[0].evidence[0].line_start, 3);
    assert!(findings[0].evidence[0].snippet.contains("TODO"));

    assert_eq!(findings[1].rule_id, "code-marker.fixme");
    assert_eq!(findings[1].severity, Severity::Medium);
    assert_eq!(findings[1].evidence[0].line_start, 4);

    assert_eq!(findings[2].rule_id, "code-marker.hack");
    assert_eq!(findings[2].severity, Severity::Medium);
    assert_eq!(findings[2].evidence[0].line_start, 5);
}
