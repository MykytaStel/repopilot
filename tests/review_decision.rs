use repopilot::review::decision::{
    ReviewDecisionGates, ReviewDecisionVerdict, ReviewGateState, decision_from_proof,
};
use repopilot::review::proof::{
    ChangeProof, ChangeProofReason, ChangeProofReasonCode, ChangeProofVerdict, ProofCoverage,
    ProofObligations, ProofScope,
};

fn proof(verdict: ChangeProofVerdict) -> ChangeProof {
    ChangeProof {
        verdict,
        reasons: Vec::new(),
        coverage: ProofCoverage {
            scope: ProofScope::Changed,
            requested_files: 1,
            analyzed_files: 1,
            excluded_files: 0,
            unsupported_files: 0,
        },
        obligations: ProofObligations {
            applicable: 0,
            satisfied: 0,
            failed: 0,
            unavailable: 0,
            unselected: 0,
            stale: 0,
        },
        contract_deltas: Vec::new(),
        capability_coverage: Vec::new(),
        intent_drift: Default::default(),
    }
}

#[test]
fn maps_change_proof_verdicts_to_one_primary_decision() {
    assert_eq!(
        decision_from_proof(
            &proof(ChangeProofVerdict::Verified),
            false,
            ReviewDecisionGates::default()
        )
        .verdict,
        ReviewDecisionVerdict::Pass
    );
    assert_eq!(
        decision_from_proof(
            &proof(ChangeProofVerdict::Review),
            false,
            ReviewDecisionGates::default()
        )
        .verdict,
        ReviewDecisionVerdict::Review
    );
    assert_eq!(
        decision_from_proof(
            &proof(ChangeProofVerdict::Broken),
            false,
            ReviewDecisionGates::default()
        )
        .verdict,
        ReviewDecisionVerdict::Block
    );
    assert_eq!(
        decision_from_proof(
            &proof(ChangeProofVerdict::NotAssessed),
            true,
            ReviewDecisionGates::default()
        )
        .verdict,
        ReviewDecisionVerdict::NotAssessed
    );
}

#[test]
fn verified_decision_has_one_action_and_separate_gate_states() {
    let decision = decision_from_proof(
        &proof(ChangeProofVerdict::Verified),
        false,
        ReviewDecisionGates {
            ci: ReviewGateState::Failed,
            review: ReviewGateState::Passed,
        },
    );

    assert_eq!(decision.verdict, ReviewDecisionVerdict::Pass);
    assert_eq!(
        decision.next_action,
        "Proceed with the normal merge review; the reported scope has compatible proof."
    );
    assert_eq!(decision.gates.ci, ReviewGateState::Failed);
    assert_eq!(decision.gates.review, ReviewGateState::Passed);
}

#[test]
fn empty_scope_explains_that_there_was_nothing_to_assess() {
    let decision = decision_from_proof(
        &proof(ChangeProofVerdict::NotAssessed),
        true,
        ReviewDecisionGates::default(),
    );

    assert_eq!(decision.verdict, ReviewDecisionVerdict::NotAssessed);
    assert_eq!(
        decision.why,
        "No changed files were available for assessment."
    );
    assert!(
        decision
            .limitations
            .iter()
            .any(|limitation| limitation.contains("No changed files"))
    );
}

#[test]
fn review_limitations_preserve_reason_messages() {
    let mut proof = proof(ChangeProofVerdict::Review);
    proof.reasons.push(ChangeProofReason::new(
        ChangeProofReasonCode::ScopeCoverageIncomplete,
        1,
        "Some requested files were excluded or unsupported.",
    ));

    let decision = decision_from_proof(&proof, false, ReviewDecisionGates::default());

    assert_eq!(decision.verdict, ReviewDecisionVerdict::Review);
    assert!(
        decision
            .limitations
            .iter()
            .any(|limitation| limitation == "Some requested files were excluded or unsupported.")
    );
}
