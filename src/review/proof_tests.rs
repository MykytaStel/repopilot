use super::*;

fn coverage(analyzed_files: usize) -> ProofCoverage {
    ProofCoverage {
        scope: ProofScope::Changed,
        requested_files: 1,
        analyzed_files,
        excluded_files: 0,
        unsupported_files: 0,
    }
}

fn obligations() -> ProofObligations {
    ProofObligations {
        applicable: 1,
        satisfied: 1,
        failed: 0,
        unavailable: 0,
        unselected: 0,
        stale: 0,
    }
}

#[test]
fn empty_scope_is_not_assessed() {
    let proof = derive_change_proof(ChangeProofInput {
        coverage: coverage(0),
        obligations: obligations(),
        sufficient_policy: true,
        broken_contracts: 0,
        reasons: Vec::new(),
    });

    assert_eq!(proof.verdict, ChangeProofVerdict::NotAssessed);
    assert_eq!(
        proof.reasons[0].code,
        ChangeProofReasonCode::ScopeNotAssessed
    );
}

#[test]
fn explicit_broken_contract_wins_over_review_reasons() {
    let proof = derive_change_proof(ChangeProofInput {
        coverage: coverage(1),
        obligations: obligations(),
        sufficient_policy: true,
        broken_contracts: 1,
        reasons: vec![ChangeProofReason::new(
            ChangeProofReasonCode::RequiredVerificationFailed,
            1,
            "a required check failed",
        )],
    });

    assert_eq!(proof.verdict, ChangeProofVerdict::Broken);
    assert!(
        proof
            .reasons
            .iter()
            .any(|reason| reason.code == ChangeProofReasonCode::BrokenContract)
    );
}

#[test]
fn incomplete_obligation_keeps_proof_at_review() {
    let proof = derive_change_proof(ChangeProofInput {
        coverage: coverage(1),
        obligations: ProofObligations {
            failed: 1,
            satisfied: 0,
            ..obligations()
        },
        sufficient_policy: true,
        broken_contracts: 0,
        reasons: Vec::new(),
    });

    assert_eq!(proof.verdict, ChangeProofVerdict::Review);
    assert!(
        proof
            .reasons
            .iter()
            .any(|reason| reason.code == ChangeProofReasonCode::RequiredVerificationFailed)
    );
}

#[test]
fn complete_sufficient_policy_is_verified() {
    let proof = derive_change_proof(ChangeProofInput {
        coverage: coverage(1),
        obligations: obligations(),
        sufficient_policy: true,
        broken_contracts: 0,
        reasons: Vec::new(),
    });

    assert_eq!(proof.verdict, ChangeProofVerdict::Verified);
    assert!(proof.reasons.is_empty());
}

#[test]
fn static_only_policy_cannot_claim_verified() {
    let proof = derive_change_proof(ChangeProofInput {
        coverage: coverage(1),
        obligations: obligations(),
        sufficient_policy: false,
        broken_contracts: 0,
        reasons: Vec::new(),
    });

    assert_eq!(proof.verdict, ChangeProofVerdict::Review);
    assert_eq!(
        proof.reasons[0].code,
        ChangeProofReasonCode::InsufficientPolicy
    );
}

#[test]
fn incomplete_obligation_accounting_cannot_verify() {
    let proof = derive_change_proof(ChangeProofInput {
        coverage: coverage(1),
        obligations: ProofObligations {
            applicable: 2,
            ..obligations()
        },
        sufficient_policy: true,
        broken_contracts: 0,
        reasons: Vec::new(),
    });

    assert_eq!(proof.verdict, ChangeProofVerdict::Review);
    assert!(proof.reasons.iter().any(|reason| {
        reason.code == ChangeProofReasonCode::RequiredVerificationCoverageIncomplete
    }));
}

#[test]
fn stale_obligation_cannot_claim_verified() {
    let proof = derive_change_proof(ChangeProofInput {
        coverage: coverage(1),
        obligations: ProofObligations {
            stale: 1,
            satisfied: 0,
            ..obligations()
        },
        sufficient_policy: true,
        broken_contracts: 0,
        reasons: Vec::new(),
    });

    assert_eq!(proof.verdict, ChangeProofVerdict::Review);
    assert!(
        proof
            .reasons
            .iter()
            .any(|reason| reason.code == ChangeProofReasonCode::RequiredVerificationStale)
    );
}
