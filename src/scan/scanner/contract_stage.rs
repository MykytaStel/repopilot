//! Finding-contract validation as a scan pipeline stage.
//!
//! Runs after risk scoring to validate that every finding satisfies the finding
//! contract, and turns any violations into scan diagnostics. Co-located with the
//! other scanner stages (`finalize`, `summary`, …) because it is only ever run as
//! part of a scan.

use crate::findings::contract::{
    FindingContractReport, FindingContractViolation, FindingContractViolationKind,
    validate_findings_contract,
};
use crate::findings::types::Finding;
use crate::scan::types::ScanDiagnostic;
use std::time::Instant;

pub struct FindingContractValidationStage {
    pub elapsed_us: u64,
    pub report: FindingContractReport,
    pub diagnostics: Vec<ScanDiagnostic>,
}

pub fn validate_finding_contract_stage(findings: &[Finding]) -> FindingContractValidationStage {
    let start = Instant::now();
    let report = validate_findings_contract(findings);
    let elapsed_us = start.elapsed().as_micros() as u64;
    let diagnostics = finding_contract_diagnostics(&report);

    FindingContractValidationStage {
        elapsed_us,
        report,
        diagnostics,
    }
}

fn finding_contract_diagnostics(report: &FindingContractReport) -> Vec<ScanDiagnostic> {
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
