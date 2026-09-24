//! The single next action a ChangeProof recommends. Ordered so the first
//! blocking condition a person can act on wins: failed checks, then
//! high-priority evidence, then missing or stale verification, then the rest.

use super::{ChangeProof, ChangeProofReasonCode, ChangeProofVerdict};

pub(crate) fn next_action_for(proof: &ChangeProof) -> &'static str {
    match proof.verdict {
        ChangeProofVerdict::Broken => {
            "Inspect the broken contract and its listed consumer before merge."
        }
        ChangeProofVerdict::Review => review_next_action(proof),
        ChangeProofVerdict::Verified => {
            "Proceed with the normal merge review; the reported scope has compatible proof."
        }
        ChangeProofVerdict::NotAssessed => {
            "Expand the analyzable scope before treating this review as evidence."
        }
    }
}

fn review_next_action(proof: &ChangeProof) -> &'static str {
    let obligations = proof.obligations;
    if obligations.failed > 0 {
        "Fix the failed required checks, then run the review again."
    } else if has_reason(proof, is_high_priority) {
        "Resolve or confirm the high-priority findings and sensitive signals listed below before merge."
    } else if obligations.unavailable > 0 {
        "Configure or install the required checks (start with repopilot init --suggestions-output repopilot-suggestions.toml), then run the review again with --verify for the chosen checks."
    } else if obligations.stale > 0 {
        "Run the required checks against the current revision, then run the review again."
    } else if obligations.unselected > 0 {
        "Select the required checks, then run the review again."
    } else if has_reason(proof, needs_human_review) {
        "Inspect the review signals and findings listed below before merge."
    } else if obligations.applicable == 0
        && has_reason(proof, |code| {
            code == ChangeProofReasonCode::InsufficientPolicy
        })
    {
        "Configure or select a proof policy (start with repopilot init --suggestions-output repopilot-suggestions.toml), then run the review again with --verify for the chosen checks."
    } else if proof.coverage.excluded_files > 0 || proof.coverage.unsupported_files > 0 {
        "Review the excluded or unsupported files before treating this review as verified."
    } else {
        "Review the listed evidence, close the proof limits, or run the required checks."
    }
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
