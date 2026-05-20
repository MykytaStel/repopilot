use crate::findings::types::{Finding, Severity};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FindingContractViolation {
    pub rule_id: String,
    pub finding_id: String,
    pub violation: FindingContractViolationKind,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum FindingContractViolationKind {
    EmptyId,
    EmptyRuleId,
    EmptyTitle,
    EmptyDescription,
    EmptyRecommendation,
    MissingEvidence,
    InvalidEvidencePath,
    InvalidEvidenceLineRange,
    MissingRiskFormulaVersion,
    MissingRiskSignals,
    MissingDocsForHighSeverity,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FindingContractReport {
    pub violations: Vec<FindingContractViolation>,
    pub findings_checked: usize,
    pub valid_findings: usize,
    pub invalid_findings: usize,
}

pub fn validate_finding_contract(finding: &Finding) -> Vec<FindingContractViolation> {
    let mut violations = Vec::new();

    push_if(
        &mut violations,
        finding,
        finding.id.trim().is_empty(),
        FindingContractViolationKind::EmptyId,
    );
    push_if(
        &mut violations,
        finding,
        finding.rule_id.trim().is_empty(),
        FindingContractViolationKind::EmptyRuleId,
    );
    push_if(
        &mut violations,
        finding,
        finding.title.trim().is_empty(),
        FindingContractViolationKind::EmptyTitle,
    );
    push_if(
        &mut violations,
        finding,
        finding.description.trim().is_empty(),
        FindingContractViolationKind::EmptyDescription,
    );
    push_if(
        &mut violations,
        finding,
        finding.recommendation.trim().is_empty(),
        FindingContractViolationKind::EmptyRecommendation,
    );

    if finding.evidence.is_empty() {
        push_violation(
            &mut violations,
            finding,
            FindingContractViolationKind::MissingEvidence,
        );
    } else {
        for evidence in &finding.evidence {
            if evidence.path.as_os_str().is_empty() {
                push_violation(
                    &mut violations,
                    finding,
                    FindingContractViolationKind::InvalidEvidencePath,
                );
            }
            if evidence.line_start == 0
                || evidence
                    .line_end
                    .is_some_and(|line_end| line_end < evidence.line_start)
            {
                push_violation(
                    &mut violations,
                    finding,
                    FindingContractViolationKind::InvalidEvidenceLineRange,
                );
            }
        }
    }

    push_if(
        &mut violations,
        finding,
        finding.risk.formula_version.trim().is_empty(),
        FindingContractViolationKind::MissingRiskFormulaVersion,
    );
    push_if(
        &mut violations,
        finding,
        finding.risk.signals.is_empty(),
        FindingContractViolationKind::MissingRiskSignals,
    );

    if matches!(finding.severity, Severity::High | Severity::Critical)
        && finding
            .docs_url
            .as_deref()
            .is_none_or(|docs_url| docs_url.trim().is_empty())
    {
        push_violation(
            &mut violations,
            finding,
            FindingContractViolationKind::MissingDocsForHighSeverity,
        );
    }

    violations
}

pub fn validate_findings_contract(findings: &[Finding]) -> FindingContractReport {
    let mut violations = Vec::new();
    let mut invalid_findings = 0;

    for finding in findings {
        let finding_violations = validate_finding_contract(finding);
        if !finding_violations.is_empty() {
            invalid_findings += 1;
        }
        violations.extend(finding_violations);
    }

    FindingContractReport {
        violations,
        findings_checked: findings.len(),
        valid_findings: findings.len().saturating_sub(invalid_findings),
        invalid_findings,
    }
}

fn push_if(
    violations: &mut Vec<FindingContractViolation>,
    finding: &Finding,
    condition: bool,
    kind: FindingContractViolationKind,
) {
    if condition {
        push_violation(violations, finding, kind);
    }
}

fn push_violation(
    violations: &mut Vec<FindingContractViolation>,
    finding: &Finding,
    kind: FindingContractViolationKind,
) {
    violations.push(FindingContractViolation {
        rule_id: finding.rule_id.clone(),
        finding_id: finding.id.clone(),
        violation: kind,
    });
}
