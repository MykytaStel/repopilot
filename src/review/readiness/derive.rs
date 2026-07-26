use super::model::{MergeReadinessRecord, ReadinessReason, ReadinessReasonCode, ReadinessVerdict};
use crate::baseline::gate::CiGateResult;
use crate::findings::decision::build_decision_record;
use crate::findings::redaction::human_verification_step;
use crate::history::RiskDelta;
use crate::review::gate::ReviewSignalGateResult;
use crate::review::model::ReviewReport;
use crate::risk::RiskPriority;
use std::collections::BTreeSet;

pub fn derive_readiness(
    report: &ReviewReport,
    ci_gate: Option<&CiGateResult>,
    review_gate: Option<&ReviewSignalGateResult>,
    risk_delta: Option<&RiskDelta>,
) -> MergeReadinessRecord {
    let mut reasons = Vec::new();
    let findings = report.in_diff_findings();
    let p0 = findings
        .iter()
        .filter(|finding| finding.risk.priority == RiskPriority::P0)
        .count();
    let p1 = findings
        .iter()
        .filter(|finding| finding.risk.priority == RiskPriority::P1)
        .count();
    let definitely = report
        .tiered_signals
        .definitely
        .iter()
        .filter(|signal| !signal.suppressed)
        .count();
    let maybe = report
        .tiered_signals
        .maybe
        .iter()
        .filter(|signal| !signal.suppressed)
        .count();

    push_if(
        &mut reasons,
        report.summary.has_error_diagnostics(),
        ReadinessReasonCode::AnalysisError,
        1,
        "Analysis completed with an error diagnostic.",
    );
    push_if(
        &mut reasons,
        ci_gate.is_some_and(|gate| !gate.passed()),
        ReadinessReasonCode::FindingGateFailed,
        ci_gate.map_or(0, |gate| gate.failed_findings),
        "The configured finding gate failed.",
    );
    push_if(
        &mut reasons,
        review_gate.is_some_and(|gate| !gate.passed()),
        ReadinessReasonCode::ReviewSignalGateFailed,
        review_gate.map_or(0, |gate| gate.failed_signals),
        "The configured review-signal gate failed.",
    );
    push_count(
        &mut reasons,
        p0,
        ReadinessReasonCode::PriorityP0,
        "P0 finding occurrence(s) affect changed code.",
    );
    push_count(
        &mut reasons,
        p1,
        ReadinessReasonCode::PriorityP1,
        "P1 finding occurrence(s) affect changed code.",
    );
    push_count(
        &mut reasons,
        definitely,
        ReadinessReasonCode::DefinitelySensitive,
        "Definitely-sensitive review signal(s) require confirmation.",
    );
    push_count(
        &mut reasons,
        maybe,
        ReadinessReasonCode::MaybeSensitive,
        "Maybe-sensitive review signal(s) are visible.",
    );
    push_if(
        &mut reasons,
        report.boundary_missing_test,
        ReadinessReasonCode::BoundaryMissingTest,
        1,
        "A changed boundary has no corresponding test change.",
    );
    push_count(
        &mut reasons,
        findings.len(),
        ReadinessReasonCode::VisibleFinding,
        "Visible finding occurrence(s) affect changed code.",
    );
    push_count(
        &mut reasons,
        report.ownership.unowned_paths.len(),
        ReadinessReasonCode::UnownedSurface,
        "Changed or impacted path(s) have no named owner.",
    );
    reasons.sort_by_key(|reason| reason.code);

    MergeReadinessRecord {
        verdict: verdict(&reasons),
        reasons,
        impact: report.impact_paths.clone(),
        ownership: report.ownership.clone(),
        verification_steps: verification_steps(report),
        limitations: limitations(report),
        risk_delta: risk_delta.cloned(),
    }
}

fn verdict(reasons: &[ReadinessReason]) -> ReadinessVerdict {
    if reasons.iter().any(|reason| {
        matches!(
            reason.code,
            ReadinessReasonCode::AnalysisError
                | ReadinessReasonCode::FindingGateFailed
                | ReadinessReasonCode::ReviewSignalGateFailed
                | ReadinessReasonCode::PriorityP0
        )
    }) {
        ReadinessVerdict::Blocked
    } else if reasons.is_empty() {
        ReadinessVerdict::Ready
    } else {
        ReadinessVerdict::Review
    }
}

fn verification_steps(report: &ReviewReport) -> Vec<String> {
    let mut steps = BTreeSet::new();
    for finding in report.in_diff_findings() {
        let Some(plan) = build_decision_record(finding).verification_plan else {
            continue;
        };
        for step in plan.steps {
            steps.insert(human_verification_step(finding, &step).into_owned());
        }
    }
    steps.into_iter().collect()
}

fn limitations(report: &ReviewReport) -> Vec<String> {
    let mut limitations = vec![
        "Readiness is derived from static local evidence; RepoPilot does not execute tests."
            .to_string(),
    ];
    if !report.ownership.unowned_paths.is_empty() {
        limitations.push(
            "Unowned paths use package or directory boundaries, not inferred people.".to_string(),
        );
    }
    limitations
}

fn push_count(
    reasons: &mut Vec<ReadinessReason>,
    count: usize,
    code: ReadinessReasonCode,
    message: &str,
) {
    push_if(reasons, count > 0, code, count, message);
}

fn push_if(
    reasons: &mut Vec<ReadinessReason>,
    condition: bool,
    code: ReadinessReasonCode,
    count: usize,
    message: &str,
) {
    if condition {
        reasons.push(ReadinessReason {
            code,
            count,
            message: message.to_string(),
        });
    }
}
