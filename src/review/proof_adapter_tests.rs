use super::proof::{
    ChangeProofReasonCode, ChangeProofVerdict, ProofScope, derive_change_proof_from_review,
};
use super::{MergeReadinessRecord, ReadinessReason, ReadinessReasonCode, ReadinessVerdict};
use crate::review::diff::{ChangeStatus, ChangedFile};
use crate::review::model::ReviewReport;
use crate::scan::types::{ScanMetadata, ScanMetrics, ScanMode, ScanSummary};
use crate::verification::{VerificationOutcome, VerificationRole, VerificationStatus};
use std::path::PathBuf;

fn report(mode: ScanMode, discovered: usize, analyzed: usize) -> ReviewReport {
    let changed_files = (0..discovered.min(1))
        .map(|_| ChangedFile {
            path: PathBuf::from("src/lib.rs"),
            status: ChangeStatus::Modified,
            ranges: Vec::new(),
            hunks: Vec::new(),
        })
        .collect();
    let mut summary = ScanSummary {
        metadata: ScanMetadata {
            mode,
            ..ScanMetadata::default()
        },
        metrics: ScanMetrics {
            files_discovered: discovered,
            files_analyzed: analyzed,
            changed_files_count: discovered.min(1),
            ..ScanMetrics::default()
        },
        ..ScanSummary::default()
    };
    summary.metadata.root_path = PathBuf::from("/repo");
    ReviewReport {
        summary,
        repo_root: PathBuf::from("/repo"),
        baseline_path: None,
        changed_files,
        blast_radius: Vec::new(),
        impact_paths: Default::default(),
        ownership: Default::default(),
        ownership_diagnostics: Vec::new(),
        boundary_signals: Vec::new(),
        boundary_missing_test: false,
        tiered_signals: Default::default(),
        timings: Default::default(),
        verification: Vec::new(),
        findings: Vec::new(),
    }
}

fn readiness(verdict: ReadinessVerdict, reasons: Vec<ReadinessReason>) -> MergeReadinessRecord {
    MergeReadinessRecord {
        verdict,
        reasons,
        impact: Default::default(),
        ownership: Default::default(),
        verification_steps: Vec::new(),
        verification: Vec::new(),
        limitations: Vec::new(),
        risk_delta: None,
    }
}

fn outcome(status: VerificationStatus, revision_compatible: bool) -> VerificationOutcome {
    VerificationOutcome {
        check_id: "unit".to_string(),
        role: VerificationRole::Test,
        status,
        duration_ms: 1,
        exit_code: Some(0),
        working_directory: ".".to_string(),
        stdout_excerpt: String::new(),
        stderr_excerpt: String::new(),
        stdout_truncated: false,
        stderr_truncated: false,
        revision_before: "before".to_string(),
        revision_after: "after".to_string(),
        revision_compatible,
        limitations: Vec::new(),
        reused: false,
    }
}

#[test]
fn assessed_ready_review_with_passed_check_maps_to_verified() {
    let mut report = report(ScanMode::Changed, 1, 1);
    report.verification = vec![outcome(VerificationStatus::Passed, true)];
    let proof =
        derive_change_proof_from_review(&report, &readiness(ReadinessVerdict::Ready, vec![]));

    assert_eq!(proof.verdict, ChangeProofVerdict::Verified);
    assert_eq!(proof.coverage.scope, ProofScope::Changed);
}

#[test]
fn static_only_ready_review_stays_at_review() {
    let report = report(ScanMode::Changed, 1, 1);
    let proof =
        derive_change_proof_from_review(&report, &readiness(ReadinessVerdict::Ready, vec![]));

    assert_eq!(proof.verdict, ChangeProofVerdict::Review);
    assert!(
        proof
            .reasons
            .iter()
            .any(|reason| reason.code == ChangeProofReasonCode::InsufficientPolicy)
    );
}

#[test]
fn blocked_verification_maps_to_review_not_broken() {
    let mut report = report(ScanMode::Changed, 1, 1);
    report.verification = vec![outcome(VerificationStatus::Failed, true)];
    let readiness = readiness(
        ReadinessVerdict::Blocked,
        vec![ReadinessReason {
            code: ReadinessReasonCode::VerificationFailed,
            count: 1,
            message: "Selected verification check(s) failed.".to_string(),
        }],
    );

    let proof = derive_change_proof_from_review(&report, &readiness);

    assert_eq!(proof.verdict, ChangeProofVerdict::Review);
    assert!(
        !proof
            .reasons
            .iter()
            .any(|reason| reason.code == ChangeProofReasonCode::BrokenContract)
    );
    assert!(
        proof
            .reasons
            .iter()
            .any(|reason| reason.code == ChangeProofReasonCode::RequiredVerificationFailed)
    );
}

#[test]
fn empty_review_scope_is_not_assessed() {
    let report = report(ScanMode::Changed, 0, 0);
    let proof =
        derive_change_proof_from_review(&report, &readiness(ReadinessVerdict::Ready, vec![]));

    assert_eq!(proof.verdict, ChangeProofVerdict::NotAssessed);
    assert!(
        proof
            .reasons
            .iter()
            .any(|reason| reason.code == ChangeProofReasonCode::ScopeNotAssessed)
    );
}

#[test]
fn full_review_uses_discovered_files_for_coverage() {
    let report = report(ScanMode::Full, 4, 3);
    let proof =
        derive_change_proof_from_review(&report, &readiness(ReadinessVerdict::Ready, vec![]));

    assert_eq!(proof.coverage.scope, ProofScope::Full);
    assert_eq!(proof.coverage.requested_files, 4);
    assert_eq!(proof.coverage.analyzed_files, 3);
}

#[test]
fn incompatible_pass_is_reported_as_stale_proof() {
    let mut report = report(ScanMode::Changed, 1, 1);
    report.verification = vec![outcome(VerificationStatus::Passed, false)];
    let readiness = readiness(
        ReadinessVerdict::Blocked,
        vec![ReadinessReason {
            code: ReadinessReasonCode::VerificationRevisionChanged,
            count: 1,
            message: "Workspace revision changed during verification.".to_string(),
        }],
    );

    let proof = derive_change_proof_from_review(&report, &readiness);

    assert!(
        proof
            .reasons
            .iter()
            .any(|reason| reason.code == ChangeProofReasonCode::RequiredVerificationStale)
    );
    assert!(
        !proof
            .reasons
            .iter()
            .any(|reason| reason.code == ChangeProofReasonCode::RequiredVerificationUnavailable)
    );
}
