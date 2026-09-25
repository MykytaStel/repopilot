use super::{EvidenceCoverageLimit, limit, plural};
use crate::review::model::ReviewReport;
use crate::review::proof::{ChangeProof, ChangeProofReason, ChangeProofReasonCode};

pub(super) fn add_verification_limits(
    limits: &mut Vec<EvidenceCoverageLimit>,
    proof: &ChangeProof,
) {
    if !proof.obligations.accounted_for() {
        limits.push(limit(
            "unaccounted-verification-checks",
            proof.obligations.applicable.saturating_sub(
                proof.obligations.satisfied
                    + proof.obligations.failed
                    + proof.obligations.unavailable
                    + proof.obligations.unselected
                    + proof.obligations.stale,
            ),
            "Verification results do not account for every applicable check.".to_string(),
        ));
    }
    add_check_limit(limits, "verification-failed", proof.obligations.failed);
    add_check_limit(
        limits,
        "verification-unavailable",
        proof.obligations.unavailable,
    );
    add_check_limit(
        limits,
        "verification-unselected",
        proof.obligations.unselected,
    );
    add_check_limit(limits, "verification-stale", proof.obligations.stale);
}

fn add_check_limit(limits: &mut Vec<EvidenceCoverageLimit>, code: &str, count: usize) {
    if count > 0 {
        limits.push(limit(code, count, verification_message(code, count)));
    }
}

pub(super) fn add_reason_limits(
    limits: &mut Vec<EvidenceCoverageLimit>,
    report: &ReviewReport,
    proof: &ChangeProof,
) {
    for reason in &proof.reasons {
        if let Some(item) = reason_limit(report, reason) {
            limits.push(item);
        }
    }
}

fn reason_limit(
    report: &ReviewReport,
    reason: &ChangeProofReason,
) -> Option<EvidenceCoverageLimit> {
    match reason.code {
        ChangeProofReasonCode::InsufficientPolicy => {
            Some(verification_policy_limit(report, reason.count))
        }
        ChangeProofReasonCode::RequiredVerificationFailed
        | ChangeProofReasonCode::RequiredVerificationUnavailable
        | ChangeProofReasonCode::RequiredVerificationUnselected
        | ChangeProofReasonCode::RequiredVerificationStale
        | ChangeProofReasonCode::RequiredVerificationCoverageIncomplete => {
            verification_reason_limit(reason)
        }
        ChangeProofReasonCode::UnsupportedContractCoverage => Some(limit(
            "unsupported-contract-coverage",
            reason.count,
            format!(
                "{} contract change{} have limited proof coverage",
                reason.count,
                plural(reason.count)
            ),
        )),
        ChangeProofReasonCode::AnalysisError => Some(limit(
            "analysis-error",
            reason.count,
            format!(
                "Analysis reported {} error{}",
                reason.count,
                plural(reason.count)
            ),
        )),
        _ => None,
    }
}

fn verification_reason_limit(reason: &ChangeProofReason) -> Option<EvidenceCoverageLimit> {
    let (code, message) = match reason.code {
        ChangeProofReasonCode::RequiredVerificationFailed => (
            "verification-failed",
            verification_message("verification-failed", reason.count),
        ),
        ChangeProofReasonCode::RequiredVerificationUnavailable => (
            "verification-unavailable",
            verification_message("verification-unavailable", reason.count),
        ),
        ChangeProofReasonCode::RequiredVerificationUnselected => (
            "verification-unselected",
            verification_message("verification-unselected", reason.count),
        ),
        ChangeProofReasonCode::RequiredVerificationStale => (
            "verification-stale",
            verification_message("verification-stale", reason.count),
        ),
        ChangeProofReasonCode::RequiredVerificationCoverageIncomplete => {
            ("verification-coverage-incomplete", reason.message.clone())
        }
        _ => return None,
    };
    Some(limit(code, reason.count, message))
}

fn verification_policy_limit(report: &ReviewReport, count: usize) -> EvidenceCoverageLimit {
    let (code, message) = if report.verification_policy.configured.is_empty()
        && report.verification_policy.selected.is_empty()
    {
        (
            "verification-not-configured",
            "Verification checks are not configured.",
        )
    } else if report.verification_policy.selected.is_empty() {
        (
            "verification-not-selected",
            "No verification checks are selected.",
        )
    } else {
        (
            "verification-policy-insufficient",
            "Selected verification checks do not cover all required changes.",
        )
    };
    limit(code, count, message)
}

fn verification_message(code: &str, count: usize) -> String {
    let (item, singular_verb, plural_verb) = match code {
        "verification-failed" => ("required verification check", "failed", "failed"),
        "verification-unavailable" => (
            "required verification check",
            "is unavailable",
            "are unavailable",
        ),
        "verification-unselected" => (
            "required verification check",
            "is not selected",
            "are not selected",
        ),
        _ => (
            "verification result",
            "is stale for this revision",
            "are stale for this revision",
        ),
    };
    let verb = if count == 1 {
        singular_verb
    } else {
        plural_verb
    };
    format!("{count} {item}{} {verb}", plural(count))
}
