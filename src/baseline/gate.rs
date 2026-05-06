use crate::baseline::diff::{BaselineScanReport, BaselineStatus};
use crate::findings::types::{Finding, Severity};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum FailOn {
    New(Severity),
    Any(Severity),
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
        }
    }

    pub fn failure_message(&self) -> Option<String> {
        if self.passed() {
            return None;
        }

        let scope = match self.fail_on {
            FailOn::New(_) => "new findings",
            FailOn::Any(_) => "findings",
        };
        let severity = match self.fail_on {
            FailOn::New(severity) | FailOn::Any(severity) => severity.lowercase_label(),
        };

        Some(format!(
            "RepoPilot CI Gate failed\n\nReason:\nFound {} {scope} at or above {severity} severity.\n\nUse `repopilot baseline create . --force` only if these findings are accepted technical debt.",
            self.failed_findings
        ))
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
    }
}
