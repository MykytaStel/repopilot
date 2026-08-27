use super::*;
use crate::baseline::model::{BASELINE_SCHEMA_VERSION, BASELINE_TOOL, Baseline, BaselineFinding};
use crate::findings::types::{Evidence, Finding, Severity};
use crate::scan::types::{ScanArtifacts, ScanMetadata, ScanSummary};
use std::path::PathBuf;

fn make_finding(rule_id: &str, path: &str, line: usize) -> Finding {
    Finding {
        id: format!("{rule_id}-test"),
        rule_id: rule_id.to_string(),
        title: "Test".to_string(),
        severity: Severity::High,
        evidence: vec![Evidence {
            path: PathBuf::from(path),
            line_start: line,
            line_end: None,
            snippet: String::new(),
        }],
        ..Default::default()
    }
}

fn make_summary(findings: Vec<Finding>) -> ScanSummary {
    ScanSummary {
        metadata: ScanMetadata {
            root_path: PathBuf::from("/project"),
            ..Default::default()
        },
        artifacts: ScanArtifacts {
            findings,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn baseline_with_keys(keys: Vec<String>) -> Baseline {
    Baseline {
        schema_version: BASELINE_SCHEMA_VERSION,
        tool: BASELINE_TOOL.to_string(),
        created_at: "2024-01-01T00:00:00Z".to_string(),
        root: "/project".to_string(),
        findings: keys
            .into_iter()
            .map(|key| BaselineFinding {
                key: key.clone(),
                rule_id: "test.rule".to_string(),
                severity: "HIGH".to_string(),
                path: "src/main.rs".to_string(),
                message: "Test".to_string(),
            })
            .collect(),
    }
}

#[test]
fn new_finding_not_in_baseline_is_marked_new() {
    let finding = make_finding("test.rule", "src/main.rs", 10);
    let summary = make_summary(vec![finding]);
    let baseline = baseline_with_keys(vec![]);
    let baseline_path = PathBuf::from(".repopilot/baseline.json");

    let report = diff_summary_against_baseline(summary, &baseline, baseline_path);

    assert_eq!(report.findings[0].status, BaselineStatus::New);
    assert_eq!(report.new_count(), 1);
    assert_eq!(report.existing_count(), 0);
}

#[test]
fn finding_in_baseline_is_marked_existing() {
    let finding = make_finding("test.rule", "src/main.rs", 10);
    let root = PathBuf::from("/project");
    let key = stable_finding_key(&finding, &root);
    let summary = make_summary(vec![finding]);
    let baseline = baseline_with_keys(vec![key]);
    let baseline_path = PathBuf::from(".repopilot/baseline.json");

    let report = diff_summary_against_baseline(summary, &baseline, baseline_path);

    assert_eq!(report.findings[0].status, BaselineStatus::Existing);
    assert_eq!(report.new_count(), 0);
    assert_eq!(report.existing_count(), 1);
}

#[test]
fn legacy_line_based_baseline_key_matches_after_line_shift() {
    let finding = make_finding("test.rule", "src/main.rs", 12);
    let summary = make_summary(vec![finding]);
    let baseline = baseline_with_keys(vec!["test.rule:src/main.rs:10".to_string()]);
    let baseline_path = PathBuf::from(".repopilot/baseline.json");

    let report = diff_summary_against_baseline(summary, &baseline, baseline_path);

    assert_eq!(report.findings[0].status, BaselineStatus::Existing);
}

#[test]
fn all_findings_new_marks_every_finding_as_new() {
    let summary = make_summary(vec![
        make_finding("rule.one", "src/a.rs", 1),
        make_finding("rule.two", "src/b.rs", 2),
    ]);

    let report = all_findings_new(summary);

    assert_eq!(report.new_count(), 2);
    assert_eq!(report.existing_count(), 0);
    assert!(report.baseline_path.is_none());
}

#[test]
fn mixed_new_and_existing_findings() {
    let new_finding = make_finding("rule.new", "src/new.rs", 5);
    let existing_finding = make_finding("rule.existing", "src/existing.rs", 10);
    let root = PathBuf::from("/project");
    let existing_key = stable_finding_key(&existing_finding, &root);

    let summary = make_summary(vec![new_finding, existing_finding]);
    let baseline = baseline_with_keys(vec![existing_key]);

    let report = diff_summary_against_baseline(
        summary,
        &baseline,
        PathBuf::from(".repopilot/baseline.json"),
    );

    assert_eq!(report.new_count(), 1);
    assert_eq!(report.existing_count(), 1);
}

#[test]
fn baseline_finding_absent_from_the_scan_is_resolved() {
    // The case the Action's own PR delta needs and the CLI baseline model
    // never reported: a finding that existed when the baseline was created
    // and is simply gone now — fixed, or its file deleted.
    let surviving_finding = make_finding("rule.survives", "src/keep.rs", 1);
    let root = PathBuf::from("/project");
    let surviving_key = stable_finding_key(&surviving_finding, &root);
    let summary = make_summary(vec![surviving_finding]);
    let baseline = baseline_with_keys(vec![surviving_key, "rule.gone:src/gone.rs".to_string()]);

    let report = diff_summary_against_baseline(
        summary,
        &baseline,
        PathBuf::from(".repopilot/baseline.json"),
    );

    assert_eq!(report.resolved_count(), 1);
    assert_eq!(report.resolved[0].key, "rule.gone:src/gone.rs");
    assert_eq!(report.new_count(), 0);
    assert_eq!(report.existing_count(), 1);
}

#[test]
fn a_finding_present_in_both_is_never_reported_resolved() {
    // The false-positive guard: an unchanged finding must not show up as both
    // Existing and Resolved.
    let finding = make_finding("test.rule", "src/main.rs", 10);
    let root = PathBuf::from("/project");
    let key = stable_finding_key(&finding, &root);
    let summary = make_summary(vec![finding]);
    let baseline = baseline_with_keys(vec![key]);

    let report = diff_summary_against_baseline(
        summary,
        &baseline,
        PathBuf::from(".repopilot/baseline.json"),
    );

    assert_eq!(report.resolved_count(), 0);
}

#[test]
fn a_legacy_line_keyed_baseline_finding_that_moved_is_not_resolved() {
    // Symmetric with `legacy_line_based_baseline_key_matches_after_line_shift`:
    // resolved detection must honor the same legacy-descriptor fallback, or
    // every pre-existing baseline written before the current key scheme would
    // report its whole contents as resolved the moment any line shifted.
    let finding = make_finding("test.rule", "src/main.rs", 12);
    let summary = make_summary(vec![finding]);
    let baseline = baseline_with_keys(vec!["test.rule:src/main.rs:10".to_string()]);

    let report = diff_summary_against_baseline(
        summary,
        &baseline,
        PathBuf::from(".repopilot/baseline.json"),
    );

    assert_eq!(report.resolved_count(), 0);
}

#[test]
fn all_findings_new_reports_nothing_resolved() {
    // There is no baseline to have resolved anything against.
    let summary = make_summary(vec![make_finding("rule.one", "src/a.rs", 1)]);

    let report = all_findings_new(summary);

    assert_eq!(report.resolved_count(), 0);
}
