use crate::findings::contract::{
    FindingContractReport, FindingContractViolation, FindingContractViolationKind,
};
use crate::scan::types::ScanDiagnostic;

pub fn finding_contract_diagnostics(report: &FindingContractReport) -> Vec<ScanDiagnostic> {
    if report.violations.is_empty() {
        return Vec::new();
    }

    let sample = report
        .violations
        .iter()
        .take(5)
        .map(describe_violation)
        .collect::<Vec<_>>()
        .join("; ");

    vec![ScanDiagnostic::warning(
        "finding.contract-violation",
        format!(
            "{} finding contract violation(s) across {} invalid finding(s): {}",
            report.violations.len(),
            report.invalid_findings,
            sample
        ),
    )]
}

fn describe_violation(violation: &FindingContractViolation) -> String {
    let rule_id = if violation.rule_id.trim().is_empty() {
        "<empty-rule>"
    } else {
        violation.rule_id.as_str()
    };
    let finding_id = if violation.finding_id.trim().is_empty() {
        "<empty-finding>"
    } else {
        violation.finding_id.as_str()
    };

    format!(
        "{rule_id}/{finding_id}: {}",
        violation_label(violation.violation)
    )
}

fn violation_label(kind: FindingContractViolationKind) -> &'static str {
    match kind {
        FindingContractViolationKind::EmptyId => "empty id",
        FindingContractViolationKind::EmptyRuleId => "empty rule id",
        FindingContractViolationKind::EmptyTitle => "empty title",
        FindingContractViolationKind::EmptyDescription => "empty description",
        FindingContractViolationKind::EmptyRecommendation => "empty recommendation",
        FindingContractViolationKind::MissingEvidence => "missing evidence",
        FindingContractViolationKind::InvalidEvidencePath => "invalid evidence path",
        FindingContractViolationKind::InvalidEvidenceLineRange => "invalid evidence line range",
        FindingContractViolationKind::MissingRiskFormulaVersion => "missing risk formula version",
        FindingContractViolationKind::MissingRiskSignals => "missing risk signals",
        FindingContractViolationKind::MissingDocsForHighSeverity => {
            "missing docs for high severity"
        }
    }
}
