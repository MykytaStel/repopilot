use crate::baseline::gate::CiGateResult;
use crate::findings::types::Finding;
use crate::review::ReviewSignalGateResult;
use crate::review::model::ReviewReport;
use crate::risk::RiskPriority;
use crate::scan::types::ScanSummary;
use std::fmt::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionVerdict {
    Pass,
    Review,
    Block,
}

impl DecisionVerdict {
    pub fn label(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Review => "REVIEW",
            Self::Block => "BLOCK",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionSummary {
    pub verdict: DecisionVerdict,
    pub headline: String,
    pub reasons: Vec<String>,
    pub findings: usize,
    pub p0: usize,
    pub p1: usize,
    pub verification_plans: usize,
    pub definitely_sensitive: usize,
    pub maybe_sensitive: usize,
    pub affected_files: usize,
}

pub fn scan_decision_summary(summary: &ScanSummary) -> DecisionSummary {
    let findings = summary.artifacts.findings.iter().collect::<Vec<_>>();
    let stats = finding_stats(&findings);
    let has_errors = summary.has_error_diagnostics();

    let verdict = if has_errors || stats.p0 > 0 {
        DecisionVerdict::Block
    } else if findings.is_empty() {
        DecisionVerdict::Pass
    } else {
        DecisionVerdict::Review
    };

    let headline = match verdict {
        DecisionVerdict::Pass => {
            "No visible findings require action in the selected profile.".to_string()
        }
        DecisionVerdict::Review => {
            "Review the prioritized findings before treating the repository as ready.".to_string()
        }
        DecisionVerdict::Block => {
            "Resolve scan errors or P0 evidence before shipping this repository.".to_string()
        }
    };

    let mut reasons = Vec::new();
    if has_errors {
        reasons.push("The scan completed with error diagnostics.".to_string());
    }
    push_priority_reasons(&mut reasons, stats.p0, stats.p1);
    if findings.is_empty() && summary.metrics.hidden_suggestions_count > 0 {
        reasons.push(format!(
            "{} strict-only suggestion(s) are hidden by the selected profile.",
            summary.metrics.hidden_suggestions_count
        ));
    }

    DecisionSummary {
        verdict,
        headline,
        reasons,
        findings: findings.len(),
        p0: stats.p0,
        p1: stats.p1,
        verification_plans: stats.verification_plans,
        definitely_sensitive: 0,
        maybe_sensitive: 0,
        affected_files: 0,
    }
}

pub fn review_decision_summary(
    report: &ReviewReport,
    ci_gate: Option<&CiGateResult>,
    review_gate: Option<&ReviewSignalGateResult>,
) -> DecisionSummary {
    let findings = report.in_diff_findings();
    let stats = finding_stats(&findings);
    let definitely_sensitive = report
        .tiered_signals
        .definitely
        .iter()
        .filter(|signal| !signal.suppressed)
        .count();
    let maybe_sensitive = report
        .tiered_signals
        .maybe
        .iter()
        .filter(|signal| !signal.suppressed)
        .count();
    let finding_gate_failed = ci_gate.is_some_and(|gate| !gate.passed());
    let review_gate_failed = review_gate.is_some_and(|gate| !gate.passed());

    let verdict = if finding_gate_failed || review_gate_failed {
        DecisionVerdict::Block
    } else if stats.p0 > 0
        || stats.p1 > 0
        || definitely_sensitive > 0
        || maybe_sensitive > 0
        || !findings.is_empty()
    {
        DecisionVerdict::Review
    } else {
        DecisionVerdict::Pass
    };

    let headline = match verdict {
        DecisionVerdict::Pass if report.changed_files.is_empty() => {
            "No changed files or visible in-diff risks were detected.".to_string()
        }
        DecisionVerdict::Pass => {
            "No visible finding or review signal requires action before merge.".to_string()
        }
        DecisionVerdict::Review => {
            "Review the changed-code decisions and sensitive signals before merge.".to_string()
        }
        DecisionVerdict::Block => {
            "An enabled finding or review-signal gate failed; do not merge yet.".to_string()
        }
    };

    let mut reasons = Vec::new();
    if finding_gate_failed {
        reasons.push("The configured finding gate failed.".to_string());
    }
    if review_gate_failed {
        reasons.push("The configured definitely-sensitive review gate failed.".to_string());
    }
    push_priority_reasons(&mut reasons, stats.p0, stats.p1);
    if definitely_sensitive > 0 {
        reasons.push(format!(
            "{definitely_sensitive} definitely-sensitive review signal(s) require confirmation."
        ));
    }
    if maybe_sensitive > 0 {
        reasons.push(format!(
            "{maybe_sensitive} maybe-sensitive review signal(s) are visible."
        ));
    }
    if report.boundary_missing_test {
        reasons.push("A code boundary changed without a corresponding test change.".to_string());
    }

    DecisionSummary {
        verdict,
        headline,
        reasons,
        findings: findings.len(),
        p0: stats.p0,
        p1: stats.p1,
        verification_plans: stats.verification_plans,
        definitely_sensitive,
        maybe_sensitive,
        affected_files: report.impact_paths.affected_surface.impacted_files,
    }
}

pub fn render_decision_summary(output: &mut String, summary: &DecisionSummary) {
    writeln!(output, "Decision: {}", summary.verdict.label()).unwrap();
    writeln!(output, "Why: {}", summary.headline).unwrap();
    writeln!(
        output,
        "Decision inputs: {} finding(s), P0 {}, P1 {}, {} verification plan(s)",
        summary.findings, summary.p0, summary.p1, summary.verification_plans
    )
    .unwrap();

    if summary.definitely_sensitive > 0 || summary.maybe_sensitive > 0 || summary.affected_files > 0
    {
        writeln!(
            output,
            "Change signals: {} definitely, {} maybe, {} affected file(s)",
            summary.definitely_sensitive, summary.maybe_sensitive, summary.affected_files
        )
        .unwrap();
    }

    if !summary.reasons.is_empty() {
        output.push_str("Reasons:\n");
        for reason in &summary.reasons {
            writeln!(output, "  - {reason}").unwrap();
        }
    }
    output.push('\n');
}

#[derive(Default)]
struct FindingStats {
    p0: usize,
    p1: usize,
    verification_plans: usize,
}

fn finding_stats(findings: &[&Finding]) -> FindingStats {
    let mut stats = FindingStats::default();
    for finding in findings {
        match finding.risk.priority {
            RiskPriority::P0 => stats.p0 += 1,
            RiskPriority::P1 => stats.p1 += 1,
            RiskPriority::P2 | RiskPriority::P3 => {}
        }
        if crate::findings::verification::build_verification_plan(finding).is_some() {
            stats.verification_plans += 1;
        }
    }
    stats
}

fn push_priority_reasons(reasons: &mut Vec<String>, p0: usize, p1: usize) {
    if p0 > 0 {
        reasons.push(format!("{p0} P0 finding(s) are visible."));
    }
    if p1 > 0 {
        reasons.push(format!("{p1} P1 finding(s) are visible."));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::findings::types::{Confidence, FindingCategory, Severity};
    use crate::risk::RiskAssessment;

    #[test]
    fn empty_scan_passes() {
        let decision = scan_decision_summary(&ScanSummary::default());
        assert_eq!(decision.verdict, DecisionVerdict::Pass);
    }

    #[test]
    fn p0_scan_blocks() {
        let mut finding = Finding {
            id: "rule:src/lib.rs:1".to_string(),
            rule_id: "rule".to_string(),
            title: "P0 example".to_string(),
            category: FindingCategory::Security,
            severity: Severity::Critical,
            confidence: Confidence::High,
            ..Finding::default()
        };
        finding.risk = RiskAssessment {
            score: 95,
            priority: RiskPriority::P0,
            ..RiskAssessment::default()
        };
        let mut summary = ScanSummary::default();
        summary.artifacts.findings.push(finding);

        let decision = scan_decision_summary(&summary);
        assert_eq!(decision.verdict, DecisionVerdict::Block);
        assert_eq!(decision.p0, 1);
    }
}
