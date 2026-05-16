use repopilot::findings::types::Finding;
use repopilot::risk::{RiskPriority, RiskSummary};

fn finding(priority: RiskPriority, score: u8) -> Finding {
    let mut finding = Finding::default();
    finding.risk.priority = priority;
    finding.risk.score = score;
    finding
}

#[test]
fn risk_summary_counts_priorities_and_average_score() {
    let findings = vec![
        finding(RiskPriority::P1, 70),
        finding(RiskPriority::P2, 50),
        finding(RiskPriority::P2, 40),
        finding(RiskPriority::P3, 10),
    ];

    let summary = RiskSummary::from_findings(&findings);

    assert_eq!(summary.total, 4);
    assert_eq!(summary.counts.p0, 0);
    assert_eq!(summary.counts.p1, 1);
    assert_eq!(summary.counts.p2, 2);
    assert_eq!(summary.counts.p3, 1);
    assert_eq!(summary.highest_priority, Some(RiskPriority::P1));
    assert_eq!(summary.average_score, 43);
}

#[test]
fn risk_summary_is_empty_for_no_findings() {
    let summary = RiskSummary::from_findings(&[]);

    assert!(summary.is_empty());
    assert_eq!(summary.total, 0);
    assert_eq!(summary.highest_priority, None);
    assert_eq!(summary.average_score, 0);
}
