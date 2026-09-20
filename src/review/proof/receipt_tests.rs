use crate::review::diff::{ChangeStatus, ChangedFile};
use crate::review::model::ReviewReport;
use crate::review::proof::{
    ChangeProof, ChangeProofReason, ChangeProofReasonCode, ChangeProofVerdict, ProofCapability,
    ProofCapabilityStatus, ProofCoverage, ProofObligations, ProofScope,
};
use crate::review::proof::{
    ReceiptReplayContext, ReceiptReplayState, build_proof_receipt, replay_receipt,
    replay_receipt_with_reason, replay_serialized_receipt,
};
use crate::review::verification::VerificationPolicy;
use crate::review::verification::VerificationPolicyCheck;
use crate::scan::types::{ScanMetadata, ScanMode, ScanSummary};
use crate::verification::VerificationRole;
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
fn receipt_bytes_are_stable_when_proof_collections_are_reordered() {
    let (report, mut first_proof) = review_and_proof(ChangeProofVerdict::Review);
    first_proof.reasons = vec![
        ChangeProofReason::new(ChangeProofReasonCode::VisibleFinding, 2, "two"),
        ChangeProofReason::new(ChangeProofReasonCode::AnalysisError, 1, "one"),
    ];
    first_proof.capability_coverage = vec![
        ProofCapability {
            id: "z".to_string(),
            status: ProofCapabilityStatus::Limited,
            count: 2,
            message: "zeta".to_string(),
        },
        ProofCapability {
            id: "a".to_string(),
            status: ProofCapabilityStatus::Assessed,
            count: 1,
            message: "alpha".to_string(),
        },
    ];
    first_proof.intent_drift.actual_paths = vec!["z.rs".to_string(), "a.rs".to_string()];
    first_proof.intent_drift.missing_verification =
        vec!["z-check".to_string(), "a-check".to_string()];
    let mut second_proof = first_proof.clone();
    second_proof.reasons.reverse();
    second_proof.capability_coverage.reverse();
    second_proof.intent_drift.actual_paths.reverse();
    second_proof.intent_drift.missing_verification.reverse();

    let first = build_proof_receipt(&report, &first_proof);
    let second = build_proof_receipt(&report, &second_proof);

    assert_eq!(first.projection_hash, second.projection_hash);
    assert_eq!(
        serde_json::to_vec(&first).expect("first receipt serializes"),
        serde_json::to_vec(&second).expect("second receipt serializes")
    );
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
fn serialized_valid_receipt_replays_to_matched() {
    let (report, proof) = review_and_proof(ChangeProofVerdict::Review);
    let receipt = build_proof_receipt(&report, &proof);
    let encoded = serde_json::to_vec(&receipt).expect("receipt serializes");

    let diagnostic = replay_serialized_receipt(&encoded, &receipt.replay_context());

    assert_eq!(diagnostic.state, ReceiptReplayState::Matched);
    assert_eq!(diagnostic.code, "receipt-matched");
}

#[test]
fn receipt_discloses_empty_and_partial_scope_without_verified_claims() {
    let (report, mut empty_proof) = review_and_proof(ChangeProofVerdict::NotAssessed);
    empty_proof.coverage.requested_files = 0;
    empty_proof.coverage.analyzed_files = 0;
    let empty = build_proof_receipt(&report, &empty_proof);
    assert_eq!(
        empty.evidence.class,
        crate::review::proof::EvidenceClass::Unknown
    );
    assert_eq!(
        empty.evidence.coverage_status,
        crate::review::proof::EvidenceCoverageStatus::Unavailable
    );

    let mut partial_proof = empty_proof;
    partial_proof.verdict = ChangeProofVerdict::Review;
    partial_proof.coverage.requested_files = 3;
    partial_proof.coverage.analyzed_files = 1;
    partial_proof.coverage.unsupported_files = 2;
    let partial = build_proof_receipt(&report, &partial_proof);
    assert_eq!(
        partial.evidence.class,
        crate::review::proof::EvidenceClass::Suspicion
    );
    assert_eq!(
        partial.evidence.coverage_status,
        crate::review::proof::EvidenceCoverageStatus::Limited
    );
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
fn configuration_hash_includes_verification_role_and_paths() {
    let (mut report, proof) = review_and_proof(ChangeProofVerdict::Review);
    report.verification_policy.configured = vec![VerificationPolicyCheck {
        id: "unit".to_string(),
        role: VerificationRole::Test,
        paths: vec!["src/**".to_string()],
    }];
    let first = build_proof_receipt(&report, &proof);

    report.verification_policy.configured[0].paths = vec!["tests/**".to_string()];
    let second = build_proof_receipt(&report, &proof);

    assert_ne!(first.configuration_hash, second.configuration_hash);
    assert_eq!(
        replay_receipt(&first, &second.replay_context()),
        ReceiptReplayState::Stale
    );
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

#[test]
fn serialized_replay_reports_invalid_and_unsupported_inputs() {
    let invalid_json = replay_serialized_receipt(
        b"{",
        &ReceiptReplayContext {
            workspace_revision: None,
            configuration_hash: None,
            analyzer_version: None,
            report_schema: None,
        },
    );
    assert_eq!(invalid_json.state, ReceiptReplayState::Invalid);
    assert_eq!(invalid_json.code, "receipt-json-invalid");

    let future_shape = replay_serialized_receipt(
        br#"{"schema_version":"9.0","not_a_receipt":true}"#,
        &ReceiptReplayContext {
            workspace_revision: None,
            configuration_hash: None,
            analyzer_version: None,
            report_schema: None,
        },
    );
    assert_eq!(future_shape.state, ReceiptReplayState::Unsupported);
    assert_eq!(future_shape.code, "receipt-schema-unsupported");

    let invalid_shape = replay_serialized_receipt(
        br#"{"schema_version":"0.1"}"#,
        &ReceiptReplayContext {
            workspace_revision: None,
            configuration_hash: None,
            analyzer_version: None,
            report_schema: None,
        },
    );
    assert_eq!(invalid_shape.state, ReceiptReplayState::Invalid);
    assert_eq!(invalid_shape.code, "receipt-schema-invalid");
}

#[test]
fn serialized_replay_rejects_oversized_input_before_parsing() {
    let oversized = vec![b' '; 1024 * 1024 + 1];
    let diagnostic = replay_serialized_receipt(
        &oversized,
        &ReceiptReplayContext {
            workspace_revision: None,
            configuration_hash: None,
            analyzer_version: None,
            report_schema: None,
        },
    );

    assert_eq!(diagnostic.state, ReceiptReplayState::Invalid);
    assert_eq!(diagnostic.code, "receipt-input-too-large");
}

fn review_and_proof(verdict: ChangeProofVerdict) -> (ReviewReport, ChangeProof) {
    let report = ReviewReport {
        analysis_revision: None,
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
