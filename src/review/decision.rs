use crate::baseline::gate::CiGateResult;
use crate::review::ReviewSignalGateResult;
use crate::review::model::ReviewReport;
use crate::review::proof::{
    ChangeProof, ChangeProofReasonCode, ChangeProofVerdict, next_action_for,
};
use crate::review::readiness::MergeReadinessRecord;
use crate::scan::types::ScanMode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReviewDecisionVerdict {
    Pass,
    Review,
    Block,
    #[default]
    NotAssessed,
}

impl ReviewDecisionVerdict {
    pub fn label(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Review => "REVIEW",
            Self::Block => "BLOCK",
            Self::NotAssessed => "NOT ASSESSED",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReviewGateState {
    Passed,
    Failed,
    Disabled,
    #[default]
    NotConfigured,
}

impl ReviewGateState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Disabled => "disabled",
            Self::NotConfigured => "not configured",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewDecisionGates {
    pub ci: ReviewGateState,
    pub review: ReviewGateState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewDecision {
    pub verdict: ReviewDecisionVerdict,
    pub meaning: String,
    pub why: String,
    pub limitations: Vec<String>,
    pub next_action: String,
    pub gates: ReviewDecisionGates,
}

pub fn derive_review_decision(
    report: &ReviewReport,
    proof: &ChangeProof,
    _readiness: &MergeReadinessRecord,
    ci_gate: Option<&CiGateResult>,
    review_gate: Option<&ReviewSignalGateResult>,
) -> ReviewDecision {
    let empty_change = report.summary.mode == ScanMode::Changed && report.changed_files.is_empty();
    decision_from_proof(
        proof,
        empty_change,
        ReviewDecisionGates {
            ci: ci_gate.map_or(ReviewGateState::NotConfigured, gate_state),
            review: review_gate.map_or(ReviewGateState::NotConfigured, review_gate_state),
        },
    )
}

pub fn decision_from_proof(
    proof: &ChangeProof,
    empty_change: bool,
    gates: ReviewDecisionGates,
) -> ReviewDecision {
    let verdict = match proof.verdict {
        ChangeProofVerdict::Verified => ReviewDecisionVerdict::Pass,
        ChangeProofVerdict::Review => ReviewDecisionVerdict::Review,
        ChangeProofVerdict::Broken => ReviewDecisionVerdict::Block,
        ChangeProofVerdict::NotAssessed => ReviewDecisionVerdict::NotAssessed,
    };
    let why = why_for(proof, empty_change);
    let meaning = meaning_for(verdict, empty_change);
    let limitations = limitations_for(proof, empty_change);
    let next_action = next_action_for(proof).to_string();
    ReviewDecision {
        verdict,
        meaning,
        why,
        limitations,
        next_action,
        gates,
    }
}

fn why_for(proof: &ChangeProof, empty_change: bool) -> String {
    match proof.verdict {
        ChangeProofVerdict::Broken => {
            "A supported contract appears broken in the changed scope.".to_string()
        }
        ChangeProofVerdict::Review => {
            "Review the listed evidence, coverage limits, and required checks.".to_string()
        }
        ChangeProofVerdict::Verified => {
            "The assessed scope satisfies the selected proof policy.".to_string()
        }
        ChangeProofVerdict::NotAssessed if empty_change => {
            "No changed files were available for assessment.".to_string()
        }
        ChangeProofVerdict::NotAssessed => {
            "No analyzable files were available for assessment.".to_string()
        }
    }
}

fn meaning_for(verdict: ReviewDecisionVerdict, empty_change: bool) -> String {
    match verdict {
        ReviewDecisionVerdict::Pass => {
            "The assessed scope satisfies the selected proof policy.".to_string()
        }
        ReviewDecisionVerdict::Review => {
            "The assessment has evidence limits that need human review.".to_string()
        }
        ReviewDecisionVerdict::Block => {
            "A supported contract appears broken in the assessed scope.".to_string()
        }
        ReviewDecisionVerdict::NotAssessed if empty_change => {
            "There were no changed files to assess.".to_string()
        }
        ReviewDecisionVerdict::NotAssessed => {
            "The requested scope could not be assessed.".to_string()
        }
    }
}

fn limitations_for(proof: &ChangeProof, empty_change: bool) -> Vec<String> {
    let mut limitations = Vec::new();
    if empty_change {
        limitations.push("No changed files were available for assessment.".to_string());
    }
    if proof.coverage.analyzed_files == 0 && !empty_change {
        limitations.push("No analyzable files were available for assessment.".to_string());
    }
    for reason in &proof.reasons {
        if is_limitation(reason.code)
            && !limitations
                .iter()
                .any(|limitation| limitation == &reason.message)
        {
            limitations.push(reason.message.clone());
        }
    }
    limitations
}

fn is_limitation(code: ChangeProofReasonCode) -> bool {
    matches!(
        code,
        ChangeProofReasonCode::ScopeNotAssessed
            | ChangeProofReasonCode::ScopeCoverageIncomplete
            | ChangeProofReasonCode::RequiredVerificationFailed
            | ChangeProofReasonCode::RequiredVerificationUnavailable
            | ChangeProofReasonCode::RequiredVerificationUnselected
            | ChangeProofReasonCode::RequiredVerificationStale
            | ChangeProofReasonCode::RequiredVerificationCoverageIncomplete
            | ChangeProofReasonCode::UnsupportedContractCoverage
            | ChangeProofReasonCode::InsufficientPolicy
            | ChangeProofReasonCode::AnalysisError
            | ChangeProofReasonCode::FindingGateFailed
            | ChangeProofReasonCode::ReviewSignalGateFailed
            | ChangeProofReasonCode::IntentDrift
    )
}

fn gate_state(gate: &CiGateResult) -> ReviewGateState {
    if gate.passed() {
        ReviewGateState::Passed
    } else {
        ReviewGateState::Failed
    }
}

fn review_gate_state(gate: &ReviewSignalGateResult) -> ReviewGateState {
    if !gate.enabled() {
        ReviewGateState::Disabled
    } else if gate.passed() {
        ReviewGateState::Passed
    } else {
        ReviewGateState::Failed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_are_stable_for_machine_and_human_surfaces() {
        assert_eq!(ReviewDecisionVerdict::Pass.label(), "PASS");
        assert_eq!(ReviewDecisionVerdict::Review.label(), "REVIEW");
        assert_eq!(ReviewDecisionVerdict::Block.label(), "BLOCK");
        assert_eq!(ReviewDecisionVerdict::NotAssessed.label(), "NOT ASSESSED");
    }
}
