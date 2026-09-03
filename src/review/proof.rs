use serde::Serialize;

use crate::review::model::ReviewReport;
use crate::review::readiness::{MergeReadinessRecord, ReadinessReasonCode};
use crate::scan::types::ScanMode;
use crate::verification::VerificationStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ChangeProofVerdict {
    Broken,
    Review,
    Verified,
    NotAssessed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeProofReasonCode {
    BrokenContract,
    ScopeNotAssessed,
    RequiredVerificationFailed,
    RequiredVerificationUnavailable,
    RequiredVerificationUnselected,
    RequiredVerificationStale,
    RequiredVerificationCoverageIncomplete,
    InsufficientPolicy,
    AnalysisError,
    FindingGateFailed,
    ReviewSignalGateFailed,
    PriorityP0,
    PriorityP1,
    DefinitelySensitive,
    MaybeSensitive,
    BoundaryMissingTest,
    VisibleFinding,
    UnownedSurface,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChangeProofReason {
    pub code: ChangeProofReasonCode,
    pub count: usize,
    pub message: String,
}

impl ChangeProofReason {
    pub fn new(code: ChangeProofReasonCode, count: usize, message: impl Into<String>) -> Self {
        Self {
            code,
            count,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProofScope {
    Changed,
    Full,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProofCoverage {
    pub scope: ProofScope,
    pub requested_files: usize,
    pub analyzed_files: usize,
    pub excluded_files: usize,
    pub unsupported_files: usize,
}

impl ProofCoverage {
    fn is_meaningful(&self) -> bool {
        self.requested_files > 0 && self.analyzed_files > 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ProofObligations {
    pub applicable: usize,
    pub satisfied: usize,
    pub failed: usize,
    pub unavailable: usize,
    pub unselected: usize,
    pub stale: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeProofInput {
    pub coverage: ProofCoverage,
    pub obligations: ProofObligations,
    pub sufficient_policy: bool,
    pub broken_contracts: usize,
    pub reasons: Vec<ChangeProofReason>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChangeProof {
    pub verdict: ChangeProofVerdict,
    pub reasons: Vec<ChangeProofReason>,
    pub coverage: ProofCoverage,
    pub obligations: ProofObligations,
}

pub fn derive_change_proof(input: ChangeProofInput) -> ChangeProof {
    let mut reasons = input.reasons;
    let verdict = if !input.coverage.is_meaningful() {
        add_reason(
            &mut reasons,
            ChangeProofReason::new(
                ChangeProofReasonCode::ScopeNotAssessed,
                input.coverage.requested_files,
                "The requested scope did not contain analyzable files.",
            ),
        );
        ChangeProofVerdict::NotAssessed
    } else if input.broken_contracts > 0 {
        add_reason(
            &mut reasons,
            ChangeProofReason::new(
                ChangeProofReasonCode::BrokenContract,
                input.broken_contracts,
                "Supported evidence proves a changed contract is broken.",
            ),
        );
        ChangeProofVerdict::Broken
    } else {
        add_obligation_reasons(&mut reasons, input.obligations);
        if !input.obligations.accounted_for() {
            add_reason(
                &mut reasons,
                ChangeProofReason::new(
                    ChangeProofReasonCode::RequiredVerificationCoverageIncomplete,
                    1,
                    "The proof obligation counts do not cover every applicable check.",
                ),
            );
        }
        if !input.sufficient_policy {
            add_reason(
                &mut reasons,
                ChangeProofReason::new(
                    ChangeProofReasonCode::InsufficientPolicy,
                    1,
                    "No sufficient proof policy was selected for this assessment.",
                ),
            );
        }
        if reasons.is_empty() {
            ChangeProofVerdict::Verified
        } else {
            ChangeProofVerdict::Review
        }
    };
    reasons.sort_by_key(|reason| reason.code);
    ChangeProof {
        verdict,
        reasons,
        coverage: input.coverage,
        obligations: input.obligations,
    }
}

/// Build the canonical proof from the current review and readiness records.
/// This adapter is intentionally additive: callers can compare the result with
/// legacy readiness before any output or exit-code projection changes.
pub fn derive_change_proof_from_review(
    report: &ReviewReport,
    readiness: &MergeReadinessRecord,
) -> ChangeProof {
    let requested_files = match report.summary.mode {
        ScanMode::Changed => report.changed_files.len(),
        ScanMode::Full => report.summary.metrics.files_discovered,
    };
    let analyzed_files = report.summary.metrics.files_analyzed;
    let (satisfied, failed, unavailable, unselected, stale) = verification_counts(report);
    let reasons = readiness
        .reasons
        .iter()
        .filter_map(map_readiness_reason)
        .collect();

    derive_change_proof(ChangeProofInput {
        coverage: ProofCoverage {
            scope: match report.summary.mode {
                ScanMode::Changed => ProofScope::Changed,
                ScanMode::Full => ProofScope::Full,
            },
            requested_files,
            analyzed_files,
            excluded_files: requested_files.saturating_sub(analyzed_files),
            unsupported_files: 0,
        },
        obligations: ProofObligations {
            applicable: report.verification.len(),
            satisfied,
            failed,
            unavailable,
            unselected,
            stale,
        },
        sufficient_policy: !report.verification.is_empty(),
        broken_contracts: 0,
        reasons,
    })
}

fn verification_counts(report: &ReviewReport) -> (usize, usize, usize, usize, usize) {
    let mut counts = (0, 0, 0, 0, 0);
    for outcome in &report.verification {
        match outcome.status {
            VerificationStatus::Passed if outcome.revision_compatible => counts.0 += 1,
            VerificationStatus::Passed => counts.4 += 1,
            VerificationStatus::Failed => counts.1 += 1,
            VerificationStatus::TimedOut
            | VerificationStatus::Unavailable
            | VerificationStatus::Cancelled => counts.2 += 1,
            VerificationStatus::Skipped => counts.3 += 1,
        }
    }
    counts
}

fn map_readiness_reason(
    reason: &crate::review::readiness::ReadinessReason,
) -> Option<ChangeProofReason> {
    let code = match reason.code {
        ReadinessReasonCode::AnalysisError => ChangeProofReasonCode::AnalysisError,
        ReadinessReasonCode::FindingGateFailed => ChangeProofReasonCode::FindingGateFailed,
        ReadinessReasonCode::ReviewSignalGateFailed => {
            ChangeProofReasonCode::ReviewSignalGateFailed
        }
        ReadinessReasonCode::PriorityP0 => ChangeProofReasonCode::PriorityP0,
        ReadinessReasonCode::PriorityP1 => ChangeProofReasonCode::PriorityP1,
        ReadinessReasonCode::DefinitelySensitive => ChangeProofReasonCode::DefinitelySensitive,
        ReadinessReasonCode::MaybeSensitive => ChangeProofReasonCode::MaybeSensitive,
        ReadinessReasonCode::BoundaryMissingTest => ChangeProofReasonCode::BoundaryMissingTest,
        ReadinessReasonCode::VisibleFinding => ChangeProofReasonCode::VisibleFinding,
        ReadinessReasonCode::UnownedSurface => ChangeProofReasonCode::UnownedSurface,
        ReadinessReasonCode::VerificationFailed
        | ReadinessReasonCode::VerificationTimedOut
        | ReadinessReasonCode::VerificationUnavailable
        | ReadinessReasonCode::VerificationCancelled
        | ReadinessReasonCode::VerificationRevisionChanged => return None,
    };
    Some(ChangeProofReason::new(
        code,
        reason.count,
        reason.message.clone(),
    ))
}

fn add_obligation_reasons(reasons: &mut Vec<ChangeProofReason>, obligations: ProofObligations) {
    if obligations.failed > 0 {
        add_reason(
            reasons,
            ChangeProofReason::new(
                ChangeProofReasonCode::RequiredVerificationFailed,
                obligations.failed,
                "A required verification check failed.",
            ),
        );
    }
    if obligations.unavailable > 0 {
        add_reason(
            reasons,
            ChangeProofReason::new(
                ChangeProofReasonCode::RequiredVerificationUnavailable,
                obligations.unavailable,
                "A required verification check was unavailable.",
            ),
        );
    }
    if obligations.unselected > 0 {
        add_reason(
            reasons,
            ChangeProofReason::new(
                ChangeProofReasonCode::RequiredVerificationUnselected,
                obligations.unselected,
                "A required verification check was not selected.",
            ),
        );
    }
    if obligations.stale > 0 {
        add_reason(
            reasons,
            ChangeProofReason::new(
                ChangeProofReasonCode::RequiredVerificationStale,
                obligations.stale,
                "A required verification check passed on a different workspace revision.",
            ),
        );
    }
}

impl ProofObligations {
    fn accounted_for(self) -> bool {
        self.satisfied + self.failed + self.unavailable + self.unselected + self.stale
            == self.applicable
    }
}

fn add_reason(reasons: &mut Vec<ChangeProofReason>, reason: ChangeProofReason) {
    if let Some(existing) = reasons.iter_mut().find(|item| item.code == reason.code) {
        existing.count += reason.count;
    } else {
        reasons.push(reason);
    }
}

#[cfg(test)]
#[path = "proof_tests.rs"]
mod tests;
