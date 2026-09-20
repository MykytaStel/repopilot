use crate::review::diff::{ChangeStatus, ChangedFile};
use crate::review::model::ReviewReport;
use crate::review::proof::{
    ChangeProof, ChangeProofVerdict, ProofCoverage, ProofObligations, ProofScope,
};
use crate::review::proof::{
    ReceiptReplayContext, ReceiptReplayState, build_proof_receipt, replay_receipt,
    replay_receipt_with_reason,
};
use crate::review::verification::VerificationPolicy;
use crate::scan::types::{ScanMetadata, ScanMode, ScanSummary};
use serde_json::Value;
use std::path::PathBuf;

#[test]
fn receipt_keeps_verdict_scope_reasons_obligations_and_next_action() {
    let (report, proof) = review_and_proof(ChangeProofVerdict::Review);
    let receipt = build_proof_receipt(&report, &proof);

    assert_eq!(receipt.proof.verdict, ChangeProofVerdict::Review);
    assert_eq!(receipt.proof.coverage, proof.coverage);
    assert_eq!(receipt.proof.obligations, proof.obligations);
    assert_eq!(
        receipt.next_action,
        "Review the listed evidence, close the proof limits, or run the required checks."
    );
    assert!(receipt.projection_hash.starts_with("sha256:"));
}

#[test]
fn receipt_hash_is_stable_when_collection_inputs_are_reordered() {
    let (mut first_report, proof) = review_and_proof(ChangeProofVerdict::Verified);
    let first = build_proof_receipt(&first_report, &proof);
    first_report.changed_files.reverse();
    let second = build_proof_receipt(&first_report, &proof);

    assert_eq!(first.projection_hash, second.projection_hash);
}

#[test]
fn receipt_round_trips_through_json_for_replay() {
    let (report, proof) = review_and_proof(ChangeProofVerdict::Review);
    let receipt = build_proof_receipt(&report, &proof);
    let encoded = serde_json::to_value(&receipt).expect("receipt serializes");
    let decoded: super::ProofReceipt = serde_json::from_value::<Value>(encoded)
        .and_then(serde_json::from_value)
        .expect("receipt deserializes");

    assert_eq!(decoded, receipt);
}

#[test]
fn replay_rejects_revision_change_as_stale() {
    let (report, proof) = review_and_proof(ChangeProofVerdict::Review);
    let receipt = build_proof_receipt(&report, &proof);
    let mut context = receipt.replay_context();
    context.workspace_revision = Some("changed-revision".to_string());

    assert_eq!(
        replay_receipt(&receipt, &context),
        ReceiptReplayState::Stale
    );
    assert_eq!(receipt.proof.verdict, ChangeProofVerdict::Review);
}

#[test]
fn replay_rejects_configuration_change_as_stale() {
    let (report, proof) = review_and_proof(ChangeProofVerdict::Review);
    let receipt = build_proof_receipt(&report, &proof);
    let mut context = receipt.replay_context();
    context.configuration_hash = Some("changed-configuration".to_string());

    assert_eq!(
        replay_receipt(&receipt, &context),
        ReceiptReplayState::Stale
    );
    assert_eq!(receipt.proof.verdict, ChangeProofVerdict::Review);
}

#[test]
fn replay_rejects_future_schema_as_unsupported() {
    let (report, proof) = review_and_proof(ChangeProofVerdict::Review);
    let mut receipt = build_proof_receipt(&report, &proof);
    receipt.schema_version = "9.0".to_string();

    assert_eq!(
        replay_receipt(&receipt, &receipt.replay_context()),
        ReceiptReplayState::Unsupported
    );
    assert_eq!(receipt.proof.verdict, ChangeProofVerdict::Review);
}

#[test]
fn replay_rejects_tampered_payload_as_invalid() {
    let (report, proof) = review_and_proof(ChangeProofVerdict::Review);
    let mut receipt = build_proof_receipt(&report, &proof);
    receipt.next_action.push_str(" tampered");

    assert_eq!(
        replay_receipt(&receipt, &receipt.replay_context()),
        ReceiptReplayState::Invalid
    );
    assert_eq!(receipt.proof.verdict, ChangeProofVerdict::Review);
}

#[test]
fn replay_reports_missing_context_as_unavailable() {
    let (report, proof) = review_and_proof(ChangeProofVerdict::Review);
    let receipt = build_proof_receipt(&report, &proof);
    let context = ReceiptReplayContext {
        workspace_revision: None,
        configuration_hash: None,
        analyzer_version: None,
        report_schema: None,
    };

    assert_eq!(
        replay_receipt(&receipt, &context),
        ReceiptReplayState::Unavailable
    );
    assert_eq!(receipt.proof.verdict, ChangeProofVerdict::Review);
}

#[test]
fn replay_reason_is_bounded_and_explains_the_state() {
    let (report, proof) = review_and_proof(ChangeProofVerdict::Review);
    let receipt = build_proof_receipt(&report, &proof);
    let diagnostic = replay_receipt_with_reason(
        &receipt,
        &ReceiptReplayContext {
            workspace_revision: None,
            configuration_hash: None,
            analyzer_version: None,
            report_schema: None,
        },
    );

    assert_eq!(diagnostic.state, ReceiptReplayState::Unavailable);
    assert_eq!(diagnostic.code, "replay-context-unavailable");
    assert!(diagnostic.reason.contains("context"));
    assert!(diagnostic.reason.len() <= 256);
}

fn review_and_proof(verdict: ChangeProofVerdict) -> (ReviewReport, ChangeProof) {
    let report = ReviewReport {
        summary: ScanSummary {
            metadata: ScanMetadata {
                mode: ScanMode::Changed,
                base_ref: Some("origin/main".to_string()),
                visibility_profile: Some("default".to_string()),
                ..Default::default()
            },
            ..Default::default()
        },
        repo_root: PathBuf::from("/repo"),
        baseline_path: None,
        changed_files: vec![changed_file("b.rs"), changed_file("a.rs")],
        blast_radius: Vec::new(),
        impact_paths: Default::default(),
        ownership: Default::default(),
        ownership_diagnostics: Vec::new(),
        boundary_signals: Vec::new(),
        boundary_missing_test: false,
        tiered_signals: Default::default(),
        timings: Default::default(),
        verification_policy: VerificationPolicy {
            configured: Vec::new(),
            selected: vec!["z".to_string(), "a".to_string()],
        },
        verification: Vec::new(),
        intent: Default::default(),
        findings: Vec::new(),
    };
    let proof = ChangeProof {
        verdict,
        reasons: Vec::new(),
        coverage: ProofCoverage {
            scope: ProofScope::Changed,
            requested_files: 2,
            analyzed_files: 2,
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
    };
    (report, proof)
}

fn changed_file(path: &str) -> ChangedFile {
    ChangedFile {
        path: PathBuf::from(path),
        status: ChangeStatus::Modified,
        ranges: Vec::new(),
        hunks: Vec::new(),
    }
}
