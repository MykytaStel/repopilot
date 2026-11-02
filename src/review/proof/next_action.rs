//! The single next action a ChangeProof recommends. Ordered so the first
//! blocking condition a person can act on wins: failed checks, then
//! high-priority evidence, then missing or stale verification, then the rest.

use super::ProofObligationStatus;
use super::{ChangeProof, ChangeProofReasonCode, ChangeProofVerdict};
use std::collections::BTreeSet;

const VERIFICATION_SETUP_ACTION: &str = "Run repopilot init --suggestions-output .repopilot/init-suggestions.toml; review suggestions, add only missing accepted checks, and fix unavailable configured ones. Rerun this review with --verify CHECK_ID, keeping its original path, revision, scope, and config.";
const FAILED_VERIFICATION_ACTION: &str =
    "Fix the failed required checks, then run the review again.";
const STALE_VERIFICATION_ACTION: &str =
    "Run the required checks against the current revision, then run the review again.";
const UNSELECTED_VERIFICATION_ACTION: &str =
    "Select the required checks, then run the review again.";

pub(crate) fn next_action_for(proof: &ChangeProof) -> String {
    match proof.verdict {
        ChangeProofVerdict::Broken => {
            "Inspect the broken contract and its listed consumer before merge.".to_string()
        }
        ChangeProofVerdict::Review => review_next_action(proof),
        ChangeProofVerdict::Verified => {
            "Proceed with the normal merge review; the reported scope has compatible proof."
                .to_string()
        }
        ChangeProofVerdict::NotAssessed => {
            "Expand the analyzable scope before treating this review as evidence.".to_string()
        }
    }
}

fn review_next_action(proof: &ChangeProof) -> String {
    let obligations = proof.obligations;
    if obligations.failed > 0 {
        verification_action(
            proof,
            ProofObligationStatus::Failed,
            "Fix the failed checks, then rerun this review with",
            FAILED_VERIFICATION_ACTION,
        )
    } else if has_reason(proof, is_high_priority) {
        "Resolve or confirm the high-priority findings and sensitive signals listed below before merge.".to_string()
    } else if obligations.unavailable > 0 {
        unavailable_verification_action(proof)
    } else if obligations.stale > 0 {
        verification_action(
            proof,
            ProofObligationStatus::Stale,
            "Rerun this review against the current revision with",
            STALE_VERIFICATION_ACTION,
        )
    } else if obligations.unselected > 0 {
        verification_action(
            proof,
            ProofObligationStatus::Unselected,
            "Rerun this review with",
            UNSELECTED_VERIFICATION_ACTION,
        )
    } else if has_reason(proof, needs_human_review) {
        "Inspect the review signals and findings listed below before merge.".to_string()
    } else if obligations.applicable == 0
        && has_reason(proof, |code| {
            code == ChangeProofReasonCode::InsufficientPolicy
        })
    {
        VERIFICATION_SETUP_ACTION.to_string()
    } else if proof.coverage.excluded_files > 0 || proof.coverage.unsupported_files > 0 {
        "Review the excluded or unsupported files before treating this review as verified."
            .to_string()
    } else {
        "Review the listed evidence, close the proof limits, or run the required checks."
            .to_string()
    }
}

fn unavailable_verification_action(proof: &ChangeProof) -> String {
    let mut has_unavailable = false;
    let mut has_unconfigured = false;
    for obligation in proof
        .obligation_groups
        .iter()
        .flat_map(|group| &group.obligations)
        .filter(|obligation| obligation.status == super::ProofObligationStatus::Unavailable)
    {
        has_unavailable = true;
        has_unconfigured |= obligation.check_ids.is_empty();
    }
    if !has_unavailable || has_unconfigured {
        VERIFICATION_SETUP_ACTION.to_string()
    } else {
        verification_action(
            proof,
            ProofObligationStatus::Unavailable,
            "Resolve why the configured checks are unavailable, then rerun this review with",
            "Resolve why the configured checks are unavailable, then rerun this review with its original path, revision, scope, and config.",
        )
    }
}

fn verification_action(
    proof: &ChangeProof,
    status: ProofObligationStatus,
    action_prefix: &str,
    fallback: &str,
) -> String {
    let check_ids = proof
        .obligation_groups
        .iter()
        .flat_map(|group| &group.obligations)
        .filter(|obligation| obligation.status == status)
        .flat_map(|obligation| obligation.check_ids.iter().cloned())
        .collect::<BTreeSet<_>>();
    if check_ids.is_empty() {
        return fallback.to_string();
    }

    let verify_flags = check_ids
        .iter()
        .map(|id| format!("--verify {id}"))
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "{action_prefix} {verify_flags}, keeping its original path, revision, scope, and config."
    )
}

fn has_reason(proof: &ChangeProof, predicate: impl Fn(ChangeProofReasonCode) -> bool) -> bool {
    proof.reasons.iter().any(|reason| predicate(reason.code))
}

/// Evidence that outranks verification setup: a passing check cannot clear it.
fn is_high_priority(code: ChangeProofReasonCode) -> bool {
    matches!(
        code,
        ChangeProofReasonCode::FindingGateFailed
            | ChangeProofReasonCode::ReviewSignalGateFailed
            | ChangeProofReasonCode::PriorityP0
            | ChangeProofReasonCode::PriorityP1
            | ChangeProofReasonCode::DefinitelySensitive
    )
}

/// Evidence a person must look at; no check selection can satisfy it.
fn needs_human_review(code: ChangeProofReasonCode) -> bool {
    is_high_priority(code)
        || matches!(
            code,
            ChangeProofReasonCode::MaybeSensitive
                | ChangeProofReasonCode::BoundaryMissingTest
                | ChangeProofReasonCode::VisibleFinding
                | ChangeProofReasonCode::IntentDrift
        )
}
