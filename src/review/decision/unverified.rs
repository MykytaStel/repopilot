//! PASS without configured verification.
//!
//! Change Proof stays `REVIEW` until a repository selects a sufficient proof
//! policy. Without any configured check that `REVIEW` says nothing about the
//! change itself, so every change would read `REVIEW`, even a one-line README
//! edit. The decision passes such a change, labeled as not verified, when the
//! only open reasons are the missing verification setup. Any review signal,
//! finding, coverage gap, limited contract, gate failure, or intent drift still
//! keeps the decision at `REVIEW`.

use super::{ReviewDecision, ReviewDecisionGates, ReviewDecisionVerdict};
use crate::review::model::ReviewReport;
use crate::review::proof::{ChangeProof, ChangeProofReasonCode, ChangeProofVerdict};

const MEANING: &str = "RepoPilot flagged nothing in the assessed scope. No verification checks are configured, so the change is not verified.";
const WHY: &str =
    "No review signals, findings, coverage limits, or failed gates in the assessed scope.";
const LIMITATION: &str = "No verification checks are configured; the change is not verified.";
const NOT_VERIFIED_REASON: &str = "Not verified: no verification checks are configured.";
const NEXT_ACTION: &str = "Nothing flagged. To verify changes with your own build and test commands, run repopilot init --suggestions-output .repopilot/init-suggestions.toml.";

pub(super) fn passes_without_verification(report: &ReviewReport, proof: &ChangeProof) -> bool {
    proof.verdict == ChangeProofVerdict::Review
        && !verification_configured(report)
        && proof
            .reasons
            .iter()
            .all(|reason| is_setup_reason(reason.code))
}

/// Whether the repository configured a verification check or recorded one.
pub fn verification_configured(report: &ReviewReport) -> bool {
    !report.verification_policy.configured.is_empty() || !report.verification.is_empty()
}

/// Reasons that only say verification is not set up for this repository.
pub fn is_setup_reason(code: ChangeProofReasonCode) -> bool {
    matches!(
        code,
        ChangeProofReasonCode::InsufficientPolicy
            | ChangeProofReasonCode::RequiredVerificationUnavailable
    )
}

pub(super) fn unverified_pass(gates: ReviewDecisionGates) -> ReviewDecision {
    ReviewDecision {
        verdict: ReviewDecisionVerdict::Pass,
        meaning: MEANING.to_string(),
        why: WHY.to_string(),
        limitations: vec![LIMITATION.to_string()],
        next_action: NEXT_ACTION.to_string(),
        gates,
    }
}

impl ReviewDecision {
    /// A `PASS` that no configured check verified: renderers label it so.
    pub fn is_unverified_pass(&self, proof: &ChangeProof) -> bool {
        self.verdict == ReviewDecisionVerdict::Pass && proof.verdict != ChangeProofVerdict::Verified
    }
}

/// The reasons a summary lists under the decision. Substantive code, signal,
/// contract, and finding reasons come first so setup/selection boilerplate
/// never pushes real issues past the 5-reason header cap.
pub fn headline_reasons<'a>(report: &ReviewReport, proof: &'a ChangeProof) -> Vec<&'a str> {
    let fold_setup = !verification_configured(report);
    let has_specific_verification_gap = proof.reasons.iter().any(|reason| {
        matches!(
            reason.code,
            ChangeProofReasonCode::RequiredVerificationUnavailable
                | ChangeProofReasonCode::RequiredVerificationUnselected
        )
    });

    let keep = |code: ChangeProofReasonCode| -> bool {
        if fold_setup && is_setup_reason(code) {
            return false;
        }
        if has_specific_verification_gap && code == ChangeProofReasonCode::InsufficientPolicy {
            return false;
        }
        true
    };

    let mut substantive = Vec::new();
    let mut verification_setup = Vec::new();
    for reason in &proof.reasons {
        if !keep(reason.code) {
            continue;
        }
        if is_verification_selection_reason(reason.code) {
            verification_setup.push(reason.message.as_str());
        } else {
            substantive.push(reason.message.as_str());
        }
    }
    substantive.extend(verification_setup);
    if fold_setup
        && proof
            .reasons
            .iter()
            .any(|reason| is_setup_reason(reason.code))
    {
        substantive.push(NOT_VERIFIED_REASON);
    }
    substantive
}

fn is_verification_selection_reason(code: ChangeProofReasonCode) -> bool {
    matches!(
        code,
        ChangeProofReasonCode::InsufficientPolicy
            | ChangeProofReasonCode::RequiredVerificationUnavailable
            | ChangeProofReasonCode::RequiredVerificationUnselected
    )
}
