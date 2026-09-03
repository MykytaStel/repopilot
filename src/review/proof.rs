use serde::Serialize;

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
    RequiredVerificationCoverageIncomplete,
    InsufficientPolicy,
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
}

impl ProofObligations {
    fn accounted_for(self) -> bool {
        self.satisfied + self.failed + self.unavailable + self.unselected == self.applicable
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
