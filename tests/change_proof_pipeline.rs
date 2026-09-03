use repopilot::config::model::RepoPilotConfig;
use repopilot::review::model::ReviewReport;
use repopilot::review::proof::{
    ChangeProofReasonCode, ChangeProofVerdict, derive_change_proof_from_review,
};
use repopilot::review::{ReadinessVerdict, build_review_report, derive_readiness};
use repopilot::scan::config::ScanConfig;
use repopilot::scan::scanner::scan_changed_with_config;
use repopilot::verification::{VerificationOutcome, VerificationRole, VerificationStatus};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::{TempDir, tempdir};

#[test]
fn real_changed_review_keeps_static_only_at_review() {
    let temp = prepared_repo();
    fs::write(
        temp.path().join("src/lib.rs"),
        "pub fn answer() -> u8 { 42 }\n",
    )
    .unwrap();

    let report = review_report(&temp);
    assert_eq!(report.summary.mode.label(), "changed");
    assert_eq!(report.summary.metrics.files_analyzed, 1);

    let readiness = derive_readiness(&report, None, None, None);
    assert_eq!(readiness.verdict, ReadinessVerdict::Ready);

    let proof = derive_change_proof_from_review(&report, &readiness);
    assert_eq!(proof.verdict, ChangeProofVerdict::Review);
    assert_eq!(proof.coverage.requested_files, 1);
    assert_eq!(proof.coverage.analyzed_files, 1);
    assert!(
        proof
            .reasons
            .iter()
            .any(|reason| reason.code == ChangeProofReasonCode::InsufficientPolicy)
    );
}

#[test]
fn real_failed_verification_stays_review_not_broken() {
    let temp = prepared_repo();
    fs::write(
        temp.path().join("src/lib.rs"),
        "pub fn answer() -> u8 { 43 }\n",
    )
    .unwrap();

    let mut report = review_report(&temp);
    report.verification = vec![failed_outcome()];
    let readiness = derive_readiness(&report, None, None, None);
    let proof = derive_change_proof_from_review(&report, &readiness);

    assert_eq!(readiness.verdict, ReadinessVerdict::Blocked);
    assert_eq!(proof.verdict, ChangeProofVerdict::Review);
    assert!(
        proof
            .reasons
            .iter()
            .any(|reason| reason.code == ChangeProofReasonCode::RequiredVerificationFailed)
    );
    assert!(
        !proof
            .reasons
            .iter()
            .any(|reason| reason.code == ChangeProofReasonCode::BrokenContract)
    );
}

#[test]
fn real_empty_changed_scope_is_not_assessed() {
    let temp = prepared_repo();
    let report = review_report(&temp);
    let readiness = derive_readiness(&report, None, None, None);
    let proof = derive_change_proof_from_review(&report, &readiness);

    assert_eq!(report.summary.mode.label(), "changed");
    assert_eq!(report.summary.metrics.files_analyzed, 0);
    assert_eq!(proof.verdict, ChangeProofVerdict::NotAssessed);
    assert!(
        proof
            .reasons
            .iter()
            .any(|reason| reason.code == ChangeProofReasonCode::ScopeNotAssessed)
    );
}

fn prepared_repo() -> TempDir {
    let temp = tempdir().unwrap();
    git(temp.path(), &["init"]);
    git(
        temp.path(),
        &["config", "user.email", "repopilot@example.invalid"],
    );
    git(temp.path(), &["config", "user.name", "RepoPilot Test"]);
    fs::create_dir_all(temp.path().join("src")).unwrap();
    fs::write(
        temp.path().join("src/lib.rs"),
        "pub fn answer() -> u8 { 41 }\n",
    )
    .unwrap();
    fs::write(temp.path().join("CODEOWNERS"), "/src/ @team\n").unwrap();
    git(temp.path(), &["add", "."]);
    git(temp.path(), &["commit", "-m", "initial"]);
    temp
}

fn review_report(temp: &TempDir) -> ReviewReport {
    let summary = scan_changed_with_config(temp.path(), &ScanConfig::default(), None).unwrap();
    build_review_report(
        summary,
        temp.path(),
        None,
        None,
        None,
        &RepoPilotConfig::default(),
    )
    .unwrap()
}

fn failed_outcome() -> VerificationOutcome {
    VerificationOutcome {
        check_id: "unit".to_string(),
        role: VerificationRole::Test,
        status: VerificationStatus::Failed,
        duration_ms: 1,
        exit_code: Some(1),
        working_directory: ".".to_string(),
        stdout_excerpt: String::new(),
        stderr_excerpt: String::new(),
        stdout_truncated: false,
        stderr_truncated: false,
        revision_before: "before".to_string(),
        revision_after: "after".to_string(),
        revision_compatible: true,
        limitations: Vec::new(),
        reused: false,
    }
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
