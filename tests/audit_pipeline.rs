use repopilot::audits::pipeline::run_audits;
use repopilot::scan::config::ScanConfig;
use repopilot::scan::facts::{FileFacts, ScanFacts};
use std::path::PathBuf;

#[test]
fn audit_pipeline_converts_file_facts_into_findings() {
    let content = (0..301)
        .map(|index| format!("fn function_{index}() {{}}"))
        .chain(std::iter::once("// TODO: split this file".to_string()))
        .collect::<Vec<_>>()
        .join("\n");

    let scan_facts = ScanFacts {
        root_path: PathBuf::from("demo"),
        files_count: 1,
        directories_count: 0,
        lines_of_code: 302,
        languages: vec![],
        files: vec![FileFacts {
            path: PathBuf::from("src/large.rs"),
            language: Some("Rust".to_string()),
            lines_of_code: 302,
            content,
        }],
    };

    let findings = run_audits(&scan_facts, &ScanConfig::default());

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule_id == "architecture.large-file")
    );

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule_id == "code-marker.todo")
    );
}
