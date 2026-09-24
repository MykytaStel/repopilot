use serde::{Deserialize, Serialize};

use crate::review::intent::{IntentDrift, evaluate_intent};
use crate::review::model::ReviewReport;
use crate::review::readiness::{MergeReadinessRecord, ReadinessReasonCode};
use crate::scan::types::ScanMode;

mod capabilities;
mod contracts;
mod evidence;
mod next_action;
mod obligations;
mod receipt;
#[cfg(test)]
#[path = "proof/receipt_tests.rs"]
mod receipt_tests;
pub use crate::review::contract::{
    ChangeProofContractDelta, ContractChangeKind, ContractConfidence, ContractFamily,
};
use capabilities::capability_coverage;
pub use capabilities::{ProofCapability, ProofCapabilityStatus};
pub use evidence::{EvidenceClass, EvidenceCoverageStatus, EvidenceProvenance, EvidenceSummary};
pub(crate) use next_action::next_action_for;
use obligations::derive_verification_obligations;
pub use receipt::{
    ProofReceipt, ReceiptReplayContext, ReceiptReplayDiagnostic, ReceiptReplayState,
    build_proof_receipt, replay_receipt, replay_receipt_with_reason, replay_serialized_receipt,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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
    UnsupportedContractCoverage,
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
    IntentDrift,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProofScope {
    Changed,
    Full,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofCoverage {
    pub scope: ProofScope,
    pub requested_files: usize,
    pub analyzed_files: usize,
    pub excluded_files: usize,
    pub unsupported_files: usize,
    /// Test, fixture, example, and generated files the scanner skips by audit
    /// policy. Disclosed, but not a coverage limit: the policy is deliberate,
    /// and diff-based review signals still see these files.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub policy_skipped_files: usize,
}

fn is_zero(value: &usize) -> bool {
    *value == 0
}

impl ProofCoverage {
    fn is_meaningful(&self) -> bool {
        self.requested_files > 0 && self.analyzed_files > 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeProof {
    pub verdict: ChangeProofVerdict,
    pub reasons: Vec<ChangeProofReason>,
    pub coverage: ProofCoverage,
    pub obligations: ProofObligations,
    pub contract_deltas: Vec<ChangeProofContractDelta>,
    pub capability_coverage: Vec<ProofCapability>,
    pub intent_drift: IntentDrift,
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
    let capability_coverage = capability_coverage(&input.coverage, input.obligations);
    ChangeProof {
        verdict,
        reasons,
        coverage: input.coverage,
        obligations: input.obligations,
        contract_deltas: Vec::new(),
        capability_coverage,
        intent_drift: IntentDrift::default(),
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
    let policy_skipped_files = report
        .summary
        .metrics
        .files_skipped_low_signal
        .min(requested_files.saturating_sub(analyzed_files + excluded_files));
    let unsupported_files =
        requested_files.saturating_sub(analyzed_files + excluded_files + policy_skipped_files);
    let contract_deltas = contracts::from_review(report);
    let limited_contract_deltas = contract_deltas
        .iter()
        .filter(|delta| {
            delta.family == ContractFamily::Delivery
                || delta.confidence == Some(ContractConfidence::Limited)
        })
        .count();
    let (obligations, sufficient_policy) =
        derive_verification_obligations(report, &contract_deltas);
    let mut reasons = readiness
        .reasons
        .iter()
        .filter_map(map_readiness_reason)
        .collect::<Vec<_>>();
    if limited_contract_deltas > 0 {
        reasons.push(ChangeProofReason::new(
            ChangeProofReasonCode::UnsupportedContractCoverage,
            limited_contract_deltas,
            "Some detected contract changes have limited semantic proof coverage.",
        ));
    }

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
            policy_skipped_files,
        },
        obligations,
        sufficient_policy,
        broken_contracts: contract_deltas
            .iter()
            .filter(|delta| delta.is_broken())
            .count(),
        reasons,
    });
    proof.contract_deltas = contract_deltas;
    let mut actual_paths = report
        .changed_files
        .iter()
        .map(|file| file.path_string())
        .collect::<Vec<_>>();
    for impact in &report.impact_paths.files {
        actual_paths.push(impact.path.to_string_lossy().replace('\\', "/"));
        actual_paths.extend(
            impact
                .direct_dependents
                .iter()
                .chain(impact.transitive_dependents.iter())
                .map(|path| path.to_string_lossy().replace('\\', "/")),
        );
    }
    proof.intent_drift = evaluate_intent(
        report.intent.contract.as_ref(),
        &actual_paths,
        &proof
            .contract_deltas
            .iter()
            .map(|delta| delta.family)
            .collect::<Vec<_>>(),
        &report.intent.critical_paths,
        &report.verification_policy.selected,
    );
    if proof.intent_drift.is_drifted() {
        add_reason(
            &mut proof.reasons,
            ChangeProofReason::new(
                ChangeProofReasonCode::IntentDrift,
                proof.intent_drift.unexpected_paths.len()
                    + proof.intent_drift.unexpected_contract_families.len()
                    + proof.intent_drift.unexpected_critical_paths.len()
                    + proof.intent_drift.missing_verification.len(),
                "Observed change impact exceeds the supplied intent contract.",
            ),
        );
        if proof.verdict == ChangeProofVerdict::Verified {
            proof.verdict = ChangeProofVerdict::Review;
        }
    }
    proof.reasons.sort_by_key(|reason| reason.code);
    proof.capability_coverage.push(ProofCapability {
        id: "contract-deltas".to_string(),
        status: if limited_contract_deltas > 0 {
            ProofCapabilityStatus::Limited
        } else {
            ProofCapabilityStatus::Assessed
        },
        count: proof.contract_deltas.len(),
        message: if limited_contract_deltas > 0 {
            format!(
                "{} detected contract change(s) have limited semantic proof coverage.",
                limited_contract_deltas
            )
        } else {
            "Supported semantic contract changes detected in the review.".to_string()
        },
    });
    proof
}

fn known_excluded_files(report: &ReviewReport, requested_files: usize) -> usize {
    let metrics = &report.summary.metrics;
    metrics
        .large_files_skipped
        .saturating_add(metrics.binary_files_skipped)
        .saturating_add(metrics.files_skipped_by_limit)
        .saturating_add(metrics.files_skipped_repopilotignore)
        .min(requested_files)
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
