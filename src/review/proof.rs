use serde::Serialize;

use crate::review::model::ReviewReport;
use crate::review::readiness::{MergeReadinessRecord, ReadinessReasonCode};
use crate::scan::types::ScanMode;
use crate::verification::VerificationStatus;

mod contracts;
pub use contracts::{ChangeProofContractDelta, ContractChangeKind, ContractFamily};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ChangeProofVerdict {
    Broken,
    Review,
    Verified,
    NotAssessed,
}

impl ChangeProofVerdict {
    pub fn label(self) -> &'static str {
        match self {
            Self::Broken => "BROKEN",
            Self::Review => "REVIEW",
            Self::Verified => "VERIFIED",
            Self::NotAssessed => "NOT ASSESSED",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeProofReasonCode {
    BrokenContract,
    ScopeNotAssessed,
    ScopeCoverageIncomplete,
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
    pub contract_deltas: Vec<ChangeProofContractDelta>,
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
        add_scope_coverage_reason(&mut reasons, &input.coverage);
        ChangeProofVerdict::Broken
    } else {
        add_scope_coverage_reason(&mut reasons, &input.coverage);
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
        contract_deltas: Vec::new(),
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
    let excluded_files = known_excluded_files(report, requested_files);
    let unsupported_files =
        requested_files.saturating_sub(analyzed_files.saturating_add(excluded_files));
    let (satisfied, failed, unavailable, unselected, stale) = verification_counts(report);
    let reasons = readiness
        .reasons
        .iter()
        .filter_map(map_readiness_reason)
        .collect();

    let contract_deltas = contracts::from_review(report);
    let mut proof = derive_change_proof(ChangeProofInput {
        coverage: ProofCoverage {
            scope: match report.summary.mode {
                ScanMode::Changed => ProofScope::Changed,
                ScanMode::Full => ProofScope::Full,
            },
            requested_files,
            analyzed_files,
            excluded_files,
            unsupported_files,
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
        broken_contracts: contract_deltas.len(),
        reasons,
    });
    proof.contract_deltas = contract_deltas;
    proof
}

fn known_excluded_files(report: &ReviewReport, requested_files: usize) -> usize {
    let metrics = &report.summary.metrics;
    metrics
        .large_files_skipped
        .saturating_add(metrics.files_skipped_low_signal)
        .saturating_add(metrics.binary_files_skipped)
        .saturating_add(metrics.files_skipped_by_limit)
        .saturating_add(metrics.files_skipped_repopilotignore)
        .min(requested_files)
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

fn add_scope_coverage_reason(reasons: &mut Vec<ChangeProofReason>, coverage: &ProofCoverage) {
    let count = coverage
        .excluded_files
        .saturating_add(coverage.unsupported_files);
    if count > 0 {
        add_reason(
            reasons,
            ChangeProofReason::new(
                ChangeProofReasonCode::ScopeCoverageIncomplete,
                count,
                "Some requested files were excluded or unsupported, so the proof does not cover the full scope.",
            ),
        );
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
