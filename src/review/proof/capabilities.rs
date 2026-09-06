use serde::Serialize;

use super::{ProofCoverage, ProofObligations};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProofCapabilityStatus {
    Assessed,
    Limited,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProofCapability {
    pub id: String,
    pub status: ProofCapabilityStatus,
    pub count: usize,
    pub message: String,
}

pub(super) fn capability_coverage(
    coverage: &ProofCoverage,
    obligations: ProofObligations,
) -> Vec<ProofCapability> {
    let mut capabilities = vec![
        ProofCapability {
            id: "scope.analyzed-files".to_string(),
            status: ProofCapabilityStatus::Assessed,
            count: coverage.analyzed_files,
            message: "Files included in the supported analysis scope.".to_string(),
        },
        ProofCapability {
            id: "scope.excluded-files".to_string(),
            status: if coverage.excluded_files > 0 {
                ProofCapabilityStatus::Limited
            } else {
                ProofCapabilityStatus::Assessed
            },
            count: coverage.excluded_files,
            message: "Files skipped by an explicit repository or scanner exclusion.".to_string(),
        },
        ProofCapability {
            id: "scope.unsupported-files".to_string(),
            status: if coverage.unsupported_files > 0 {
                ProofCapabilityStatus::Unavailable
            } else {
                ProofCapabilityStatus::Assessed
            },
            count: coverage.unsupported_files,
            message: "Requested files without a supported analysis result.".to_string(),
        },
    ];
    let unresolved = obligations
        .failed
        .saturating_add(obligations.unavailable)
        .saturating_add(obligations.unselected)
        .saturating_add(obligations.stale);
    capabilities.push(ProofCapability {
        id: "verification".to_string(),
        status: if obligations.applicable == 0 {
            ProofCapabilityStatus::Unavailable
        } else if unresolved > 0 {
            ProofCapabilityStatus::Limited
        } else {
            ProofCapabilityStatus::Assessed
        },
        count: if unresolved > 0 {
            unresolved
        } else {
            obligations.satisfied
        },
        message: if obligations.applicable == 0 {
            "No verification obligation was selected for this assessment.".to_string()
        } else {
            "Selected verification obligations and their revision state.".to_string()
        },
    });
    capabilities
}
