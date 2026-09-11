use super::proof::{
    ChangeProofReasonCode, ChangeProofVerdict, ProofScope, derive_change_proof_from_review,
};
use super::{MergeReadinessRecord, ReadinessReason, ReadinessReasonCode, ReadinessVerdict};
use crate::findings::provenance::AnalysisScope;
use crate::findings::types::Confidence;
use crate::review::diff::{ChangeStatus, ChangedFile};
use crate::review::model::ReviewReport;
use crate::review::signals::tiered::{
    ConfidenceTier, ReviewSignal, ReviewSignalProvenance, ReviewSignalVerificationPlan,
    SignalFamily,
};
use crate::review::verification::{VerificationPolicy, VerificationPolicyCheck};
use crate::rules::{RuleLifecycle, SignalSource};
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
        verification_policy: Default::default(),
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

#[test]
fn signal_role_requirement_is_unselected_until_matching_check_is_selected() {
    let mut report = report(ScanMode::Changed, 1, 1);
    report
        .tiered_signals
        .definitely
        .push(access_control_signal());
    report.verification_policy = VerificationPolicy {
        configured: vec![VerificationPolicyCheck {
            id: "unit".to_string(),
            role: VerificationRole::Test,
            paths: Vec::new(),
        }],
        selected: Vec::new(),
    };

    let proof =
        derive_change_proof_from_review(&report, &readiness(ReadinessVerdict::Ready, vec![]));

    assert_eq!(proof.obligations.applicable, 1);
    assert_eq!(proof.obligations.unselected, 1);
    assert_eq!(proof.obligations.unavailable, 0);
}

#[test]
fn signal_role_requirement_is_satisfied_by_selected_matching_check() {
    let mut report = report(ScanMode::Changed, 1, 1);
    report
        .tiered_signals
        .definitely
        .push(access_control_signal());
    report.verification_policy = VerificationPolicy {
        configured: vec![VerificationPolicyCheck {
            id: "unit".to_string(),
            role: VerificationRole::Test,
            paths: Vec::new(),
        }],
        selected: vec!["unit".to_string()],
    };
    report.verification = vec![outcome(VerificationStatus::Passed, true)];

    let proof =
        derive_change_proof_from_review(&report, &readiness(ReadinessVerdict::Ready, vec![]));

    assert_eq!(proof.obligations.applicable, 1);
    assert_eq!(proof.obligations.satisfied, 1);
    assert_eq!(proof.obligations.unselected, 0);
    assert_eq!(proof.obligations.unavailable, 0);
    assert!(
        proof
            .capability_coverage
            .iter()
            .any(|capability| { capability.id == "verification" && capability.count == 1 })
    );
}

#[test]
fn signal_role_requirement_is_unavailable_without_matching_capability() {
    let mut report = report(ScanMode::Changed, 1, 1);
    report
        .tiered_signals
        .definitely
        .push(access_control_signal());

    let proof =
        derive_change_proof_from_review(&report, &readiness(ReadinessVerdict::Ready, vec![]));

    assert_eq!(proof.obligations.applicable, 1);
    assert_eq!(proof.obligations.unavailable, 1);
    assert_eq!(proof.obligations.unselected, 0);
}

#[test]
fn role_requirement_without_path_matching_capability_is_unavailable() {
    let mut report = report(ScanMode::Changed, 1, 1);
    report
        .tiered_signals
        .definitely
        .push(access_control_signal());
    report.verification_policy = VerificationPolicy {
        configured: vec![VerificationPolicyCheck {
            id: "unit".to_string(),
            role: VerificationRole::Test,
            paths: vec!["tests/**".to_string()],
        }],
        selected: Vec::new(),
    };

    let proof =
        derive_change_proof_from_review(&report, &readiness(ReadinessVerdict::Ready, vec![]));

    assert_eq!(proof.obligations.applicable, 1);
    assert_eq!(proof.obligations.unavailable, 1);
    assert_eq!(proof.obligations.unselected, 0);
}

#[test]
fn removed_export_contract_creates_typecheck_obligation_without_signal_plan() {
    let mut report = report(ScanMode::Changed, 1, 1);
    let mut signal = access_control_signal();
    signal.kind = "behavioral.removed-export-still-imported".to_string();
    signal.family = SignalFamily::Behavioral;
    signal.target_path = Some("src/api.ts".to_string());
    signal.path = "src/app.ts".to_string();
    signal.verification_plan = None;
    report.tiered_signals.definitely.push(signal);
    report.verification_policy = VerificationPolicy {
        configured: vec![VerificationPolicyCheck {
            id: "types".to_string(),
            role: VerificationRole::TypeCheck,
            paths: Vec::new(),
        }],
        selected: Vec::new(),
    };

    let proof =
        derive_change_proof_from_review(&report, &readiness(ReadinessVerdict::Ready, vec![]));

    assert_eq!(proof.contract_deltas.len(), 1);
    assert_eq!(proof.obligations.applicable, 1);
    assert_eq!(proof.obligations.unselected, 1);
}

fn access_control_signal() -> ReviewSignal {
    ReviewSignal {
        signal_id: "boundary:src/lib.rs:access-control".to_string(),
        kind: "boundary.access-control".to_string(),
        family: SignalFamily::Boundary,
        tier: ConfidenceTier::DefinitelySensitive,
        confidence: Confidence::High,
        path: "src/lib.rs".to_string(),
        target_path: None,
        line: Some(4),
        line_start: Some(4),
        line_end: Some(4),
        evidence_lines: vec![4],
        headline: "access control changed".to_string(),
        detail: None,
        blast_radius: 0,
        provenance: ReviewSignalProvenance {
            detector: "test".to_string(),
            lifecycle: RuleLifecycle::Stable,
            signal_source: SignalSource::GitDiff,
            analysis_scope: AnalysisScope::GitDiff,
        },
        suppressed: false,
        suppression_reason: None,
        gate_eligible: true,
        verification_plan: Some(ReviewSignalVerificationPlan {
            steps: vec!["run focused test".to_string()],
        }),
    }
}
