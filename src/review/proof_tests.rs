use super::*;

fn coverage(analyzed_files: usize) -> ProofCoverage {
    ProofCoverage {
        scope: ProofScope::Changed,
        requested_files: 1,
        analyzed_files,
        excluded_files: 0,
        unsupported_files: 0,
        policy_skipped_files: 0,
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
fn next_action_names_failed_required_checks() {
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

    assert_eq!(
        next_action_for(&proof),
        "Fix the failed required checks, then run the review again."
    );
}

#[test]
fn next_action_names_unavailable_required_checks() {
    let proof = derive_change_proof(ChangeProofInput {
        coverage: coverage(1),
        obligations: ProofObligations {
            unavailable: 1,
            satisfied: 0,
            ..obligations()
        },
        sufficient_policy: true,
        broken_contracts: 0,
        reasons: Vec::new(),
    });

    assert_eq!(
        next_action_for(&proof),
        "Make the required checks available, then run the review again."
    );
}

#[test]
fn next_action_names_stale_required_checks() {
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

    assert_eq!(
        next_action_for(&proof),
        "Run the required checks against the current revision, then run the review again."
    );
}

#[test]
fn next_action_names_unselected_required_checks() {
    let proof = derive_change_proof(ChangeProofInput {
        coverage: coverage(1),
        obligations: ProofObligations {
            unselected: 1,
            satisfied: 0,
            ..obligations()
        },
        sufficient_policy: true,
        broken_contracts: 0,
        reasons: Vec::new(),
    });

    assert_eq!(
        next_action_for(&proof),
        "Select the required checks, then run the review again."
    );
}

#[test]
fn next_action_explains_missing_policy_when_no_checks_apply() {
    let proof = derive_change_proof(ChangeProofInput {
        coverage: coverage(1),
        obligations: ProofObligations {
            applicable: 0,
            satisfied: 0,
            ..obligations()
        },
        sufficient_policy: false,
        broken_contracts: 0,
        reasons: Vec::new(),
    });

    assert_eq!(
        next_action_for(&proof),
        "Configure or select a proof policy (start with repopilot init --suggestions-output repopilot-suggestions.toml), then run the review again with --verify for the chosen checks."
    );
}

#[test]
fn missing_policy_outranks_coverage_limits_in_next_action() {
    let proof = derive_change_proof(ChangeProofInput {
        coverage: ProofCoverage {
            excluded_files: 1,
            requested_files: 2,
            ..coverage(1)
        },
        obligations: ProofObligations {
            applicable: 0,
            satisfied: 0,
            ..obligations()
        },
        sufficient_policy: false,
        broken_contracts: 0,
        reasons: Vec::new(),
    });

    assert!(next_action_for(&proof).starts_with("Configure or select a proof policy"));
}

#[test]
fn human_review_evidence_outranks_policy_setup_in_next_action() {
    let proof = derive_change_proof(ChangeProofInput {
        coverage: coverage(1),
        obligations: ProofObligations {
            applicable: 0,
            satisfied: 0,
            ..obligations()
        },
        sufficient_policy: false,
        broken_contracts: 0,
        reasons: vec![ChangeProofReason::new(
            ChangeProofReasonCode::MaybeSensitive,
            1,
            "Maybe-sensitive review signal(s) are visible.",
        )],
    });

    assert_eq!(
        next_action_for(&proof),
        "Inspect the review signals and findings listed below before merge."
    );
}

#[test]
fn policy_skipped_files_do_not_limit_coverage() {
    let proof = derive_change_proof(ChangeProofInput {
        coverage: ProofCoverage {
            requested_files: 3,
            policy_skipped_files: 2,
            ..coverage(1)
        },
        obligations: obligations(),
        sufficient_policy: true,
        broken_contracts: 0,
        reasons: Vec::new(),
    });

    assert_eq!(proof.verdict, ChangeProofVerdict::Verified);
    assert!(
        !proof
            .reasons
            .iter()
            .any(|reason| reason.code == ChangeProofReasonCode::ScopeCoverageIncomplete)
    );
    assert!(proof.capability_coverage.iter().any(|item| {
        item.id == "scope.policy-skipped-files"
            && item.status == ProofCapabilityStatus::Assessed
            && item.count == 2
    }));
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
    assert!(proof.capability_coverage.iter().any(|item| {
        item.id == "scope.analyzed-files"
            && item.status == ProofCapabilityStatus::Assessed
            && item.count == 1
    }));
    assert!(proof.capability_coverage.iter().any(|item| {
        item.id == "verification"
            && item.status == ProofCapabilityStatus::Assessed
            && item.count == 1
    }));
}

#[test]
fn incomplete_scope_coverage_keeps_proof_at_review() {
    let proof = derive_change_proof(ChangeProofInput {
        coverage: ProofCoverage {
            scope: ProofScope::Changed,
            requested_files: 3,
            analyzed_files: 2,
            excluded_files: 1,
            unsupported_files: 0,
            policy_skipped_files: 0,
        },
        obligations: obligations(),
        sufficient_policy: true,
        broken_contracts: 0,
        reasons: Vec::new(),
    });

    assert_eq!(proof.verdict, ChangeProofVerdict::Review);
    assert!(proof.reasons.iter().any(|reason| {
        reason.code == ChangeProofReasonCode::ScopeCoverageIncomplete && reason.count == 1
    }));
    assert!(proof.capability_coverage.iter().any(|item| {
        item.id == "scope.excluded-files"
            && item.status == ProofCapabilityStatus::Limited
            && item.count == 1
    }));
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
