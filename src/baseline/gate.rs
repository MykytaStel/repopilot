use crate::baseline::diff::{BaselineScanReport, BaselineStatus};
use crate::findings::types::{Finding, Severity};
use crate::risk::RiskPriority;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum FailOn {
    New(Severity),
    Any(Severity),
    Priority(RiskPriority),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CiGateResult {
    pub fail_on: FailOn,
    pub failed_findings: usize,
}

impl CiGateResult {
    pub fn passed(&self) -> bool {
        self.failed_findings == 0
    }

    pub fn label(&self) -> String {
        match self.fail_on {
            FailOn::New(severity) => format!("new-{}", severity.lowercase_label()),
            FailOn::Any(severity) => severity.lowercase_label().to_string(),
            FailOn::Priority(priority) => format!("priority-{}", priority.label().to_lowercase()),
        }
    }

    pub fn failure_message(&self) -> Option<String> {
        if self.passed() {
            return None;
        }

        match self.fail_on {
            FailOn::New(severity) | FailOn::Any(severity) => {
                let scope = match self.fail_on {
                    FailOn::New(_) => "new findings",
                    FailOn::Any(_) => "findings",
                    FailOn::Priority(_) => unreachable!("handled by outer match"),
                };
                Some(format!(
                    "RepoPilot CI Gate failed\n\nReason:\nFound {} {scope} at or above {} severity.\n\nUse `repopilot baseline create . --force` only if these findings are accepted technical debt.",
                    self.failed_findings,
                    severity.lowercase_label()
                ))
            }
            FailOn::Priority(priority) => Some(format!(
                "RepoPilot CI Gate failed\n\nReason:\nFound {} findings at or above {} risk priority.\n\nUse `--min-priority` to inspect the same prioritized scope locally, or lower the gate only if this risk is accepted.",
                self.failed_findings,
                priority.label()
            )),
        }
    }
}

pub fn evaluate_ci_gate(report: &BaselineScanReport, fail_on: FailOn) -> CiGateResult {
    let failed_findings = report
        .summary
        .findings
        .iter()
        .enumerate()
        .filter(|(index, finding)| finding_matches(report, *index, finding, fail_on))
        .count();

    CiGateResult {
        fail_on,
        failed_findings,
    }
}

fn finding_matches(
    report: &BaselineScanReport,
    index: usize,
    finding: &Finding,
    fail_on: FailOn,
) -> bool {
    match fail_on {
        FailOn::New(threshold) => {
            report.finding_status(index) == BaselineStatus::New
                && finding.severity.is_at_least(&threshold)
        }
        FailOn::Any(threshold) => finding.severity.is_at_least(&threshold),
        FailOn::Priority(threshold) => finding.risk.priority.is_at_least(threshold),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::baseline::diff::{BaselineScanReport, BaselineStatus, FindingBaselineStatus};
    use crate::findings::types::{Evidence, Finding, Severity};
    use crate::risk::RiskPriority;
    use crate::scan::types::ScanSummary;
    use std::path::PathBuf;

    fn make_finding(severity: Severity) -> Finding {
        Finding {
            id: "test-id".to_string(),
            rule_id: "test.rule".to_string(),
            title: "Test finding".to_string(),
            severity,
            evidence: vec![Evidence {
                path: PathBuf::from("src/main.rs"),
                line_start: 1,
                line_end: None,
                snippet: String::new(),
            }],
            ..Default::default()
        }
    }

    fn make_priority_finding(priority: RiskPriority, score: u8) -> Finding {
        let mut finding = make_finding(Severity::Low);
        finding.risk.priority = priority;
        finding.risk.score = score;
        finding
    }

    fn make_report(findings: Vec<Finding>, statuses: Vec<BaselineStatus>) -> BaselineScanReport {
        let finding_statuses = findings
            .iter()
            .zip(statuses)
            .map(|(f, status)| FindingBaselineStatus {
                key: f.rule_id.clone(),
                status,
            })
            .collect();

        BaselineScanReport {
            summary: ScanSummary {
                hidden_suggestions: Vec::new(),
                root_path: PathBuf::from("."),
                findings,
                ..Default::default()
            },
            baseline_path: Some(PathBuf::from(".repopilot/baseline.json")),
            findings: finding_statuses,
        }
    }

    #[test]
    fn gate_passes_when_no_findings_exceed_threshold() {
        let report = make_report(vec![make_finding(Severity::Low)], vec![BaselineStatus::New]);

        let result = evaluate_ci_gate(&report, FailOn::New(Severity::High));

        assert!(result.passed());
    }

    #[test]
    fn gate_fails_on_new_high_finding() {
        let report = make_report(
            vec![make_finding(Severity::High)],
            vec![BaselineStatus::New],
        );

        let result = evaluate_ci_gate(&report, FailOn::New(Severity::High));

        assert!(!result.passed());
        assert_eq!(result.failed_findings, 1);
    }

    #[test]
    fn gate_passes_when_high_finding_is_existing() {
        let report = make_report(
            vec![make_finding(Severity::High)],
            vec![BaselineStatus::Existing],
        );

        let result = evaluate_ci_gate(&report, FailOn::New(Severity::High));

        assert!(result.passed());
    }

    #[test]
    fn gate_any_mode_fails_on_existing_finding_above_threshold() {
        let report = make_report(
            vec![make_finding(Severity::Critical)],
            vec![BaselineStatus::Existing],
        );

        let result = evaluate_ci_gate(&report, FailOn::Any(Severity::High));

        assert!(!result.passed());
        assert_eq!(result.failed_findings, 1);
    }

    #[test]
    fn priority_gate_fails_on_p1_or_higher_priority() {
        let report = make_report(
            vec![make_priority_finding(RiskPriority::P1, 70)],
            vec![BaselineStatus::Existing],
        );

        let result = evaluate_ci_gate(&report, FailOn::Priority(RiskPriority::P1));

        assert!(!result.passed());
        assert_eq!(result.failed_findings, 1);
        assert_eq!(result.label(), "priority-p1");
    }

    #[test]
    fn priority_gate_passes_when_findings_are_below_threshold() {
        let report = make_report(
            vec![make_priority_finding(RiskPriority::P3, 20)],
            vec![BaselineStatus::New],
        );

        let result = evaluate_ci_gate(&report, FailOn::Priority(RiskPriority::P1));

        assert!(result.passed());
    }

    #[test]
    fn gate_passes_with_no_findings() {
        let report = make_report(vec![], vec![]);

        let result = evaluate_ci_gate(&report, FailOn::New(Severity::Low));

        assert!(result.passed());
    }
}
