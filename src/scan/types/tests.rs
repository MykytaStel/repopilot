use super::*;
use crate::findings::types::{Finding, Severity};

fn finding_with_severity(severity: Severity) -> Finding {
    Finding {
        severity,
        ..Default::default()
    }
}

#[test]
fn health_score_is_100_for_no_findings() {
    let score = ScanSummary::compute_health_score(&[], 10_000);
    assert_eq!(score, 100);
}

#[test]
fn health_score_degrades_with_critical_findings() {
    let findings = vec![finding_with_severity(Severity::Critical)];
    let score = ScanSummary::compute_health_score(&findings, 10_000);
    assert!(score < 100);
}

#[test]
fn health_score_is_clamped_to_zero() {
    let findings: Vec<Finding> = (0..50)
        .map(|_| finding_with_severity(Severity::Critical))
        .collect();
    let score = ScanSummary::compute_health_score(&findings, 1_000);
    assert_eq!(score, 0);
}

#[test]
fn health_score_same_findings_higher_for_larger_codebase() {
    let findings = vec![
        finding_with_severity(Severity::High),
        finding_with_severity(Severity::High),
    ];
    let score_small = ScanSummary::compute_health_score(&findings, 2_000);
    let score_large = ScanSummary::compute_health_score(&findings, 100_000);
    assert!(score_large > score_small);
}

#[test]
fn info_findings_do_not_reduce_score() {
    let findings = vec![
        finding_with_severity(Severity::Info),
        finding_with_severity(Severity::Info),
    ];
    let score = ScanSummary::compute_health_score(&findings, 10_000);
    assert_eq!(score, 100);
}
