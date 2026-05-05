use repopilot::audits::architecture::large_file::detect_large_file_finding;
use repopilot::findings::types::{FindingCategory, Severity};
use repopilot::scan::config::ScanConfig;
use std::path::Path;

#[test]
fn does_not_create_finding_when_file_is_under_threshold() {
    let config = ScanConfig::default();

    let finding = detect_large_file_finding(
        Path::new("src/small.rs"),
        config.large_file_loc_threshold,
        &config,
    );

    assert!(finding.is_none());
}

#[test]
fn creates_architecture_finding_when_file_is_above_threshold() {
    let config = ScanConfig::default();
    let lines_of_code = config.large_file_loc_threshold + 1;

    let finding = detect_large_file_finding(Path::new("src/large.rs"), lines_of_code, &config)
        .expect("expected large file finding");

    assert_eq!(finding.rule_id, "architecture.large-file");
    assert_eq!(finding.category, FindingCategory::Architecture);
    assert_eq!(finding.severity, Severity::Medium);
    assert_eq!(finding.evidence.len(), 1);

    let evidence = &finding.evidence[0];

    assert_eq!(evidence.path, Path::new("src/large.rs"));
    assert_eq!(evidence.line_start, 1);
    assert!(evidence.snippet.contains("301"));
    assert!(evidence.snippet.contains("configured threshold"));
}

#[test]
fn uses_high_severity_for_very_large_files() {
    let config = ScanConfig::default();

    let finding = detect_large_file_finding(
        Path::new("src/huge.rs"),
        config.huge_file_loc_threshold,
        &config,
    )
    .expect("expected huge file finding");

    assert_eq!(finding.severity, Severity::High);
}

#[test]
fn respects_custom_large_file_threshold() {
    let config = ScanConfig::default().with_large_file_loc_threshold(500);

    let below_custom_threshold =
        detect_large_file_finding(Path::new("src/not_large.rs"), 400, &config);

    let above_custom_threshold = detect_large_file_finding(Path::new("src/large.rs"), 501, &config);

    assert!(below_custom_threshold.is_none());
    assert!(above_custom_threshold.is_some());
}
