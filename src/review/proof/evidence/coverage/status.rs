use super::super::{EvidenceClass, EvidenceCoverageStatus};
use crate::review::proof::{ChangeProof, ChangeProofReasonCode, ProofCapabilityStatus};

pub(crate) fn coverage_status(proof: &ChangeProof) -> EvidenceCoverageStatus {
    if proof.coverage.analyzed_files == 0 {
        return EvidenceCoverageStatus::Unavailable;
    }
    let capability_gap = proof.capability_coverage.iter().any(|capability| {
        capability.count > 0
            && matches!(
                capability.status,
                ProofCapabilityStatus::Limited | ProofCapabilityStatus::Unavailable
            )
    });
    let accounted_files = proof
        .coverage
        .analyzed_files
        .saturating_add(proof.coverage.excluded_files)
        .saturating_add(proof.coverage.unsupported_files)
        .saturating_add(proof.coverage.policy_skipped_files);
    if proof.coverage.excluded_files > 0
        || proof.coverage.unsupported_files > 0
        || accounted_files != proof.coverage.requested_files
        || !proof.obligations.accounted_for()
        || capability_gap
        || proof
            .reasons
            .iter()
            .any(|reason| reason.code == ChangeProofReasonCode::InsufficientPolicy)
    {
        EvidenceCoverageStatus::Limited
    } else {
        EvidenceCoverageStatus::Complete
    }
}

pub(crate) fn classify(proof: &ChangeProof) -> EvidenceClass {
    match coverage_status(proof) {
        EvidenceCoverageStatus::Unavailable => EvidenceClass::Unknown,
        EvidenceCoverageStatus::Limited => {
            if proof.coverage.analyzed_files > 0 {
                EvidenceClass::Suspicion
            } else {
                EvidenceClass::Unknown
            }
        }
        EvidenceCoverageStatus::Complete => match proof.verdict {
            super::super::super::ChangeProofVerdict::Broken
            | super::super::super::ChangeProofVerdict::Verified => EvidenceClass::SupportedProof,
            super::super::super::ChangeProofVerdict::Review => EvidenceClass::Suspicion,
            super::super::super::ChangeProofVerdict::NotAssessed => EvidenceClass::Unknown,
        },
    }
}
