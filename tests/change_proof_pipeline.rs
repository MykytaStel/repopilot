use repopilot::config::model::{CriticalPathRule, RepoPilotConfig};
use repopilot::review::intent::{IntentContract, IntentStatus};
use repopilot::review::model::ReviewReport;
use repopilot::review::proof::{
    ChangeProofReasonCode, ChangeProofVerdict, EvidenceSummary, derive_change_proof_from_review,
};
use repopilot::review::{ReadinessVerdict, build_review_report, derive_readiness};
use repopilot::scan::config::ScanConfig;
use repopilot::scan::scanner::{scan_changed_with_config, scan_path_with_config};
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
fn partial_scope_is_review_with_explicit_coverage_limits() {
    let temp = prepared_repo();
    fs::write(
        temp.path().join("src/lib.rs"),
        "pub fn answer() -> u8 { 42 }\n",
    )
    .unwrap();

    let mut report = review_report(&temp);
    let extra_changed_file = report.changed_files[0].clone();
    report.changed_files.push(extra_changed_file);
    report.summary.metrics.large_files_skipped = 1;
    let readiness = derive_readiness(&report, None, None, None);
    let proof = derive_change_proof_from_review(&report, &readiness);

    assert_eq!(proof.verdict, ChangeProofVerdict::Review);
    assert_eq!(proof.coverage.requested_files, 2);
    assert_eq!(proof.coverage.analyzed_files, 1);
    assert_eq!(proof.coverage.excluded_files, 1);
    assert_eq!(proof.coverage.unsupported_files, 0);
    assert!(proof.reasons.iter().any(|reason| {
        reason.code == ChangeProofReasonCode::ScopeCoverageIncomplete && reason.count == 1
    }));
    let console = repopilot::review::render::render_console(&report, None);
    assert!(console.contains("Proof limits: 1 excluded, 0 unsupported file(s)"));
}

#[test]
fn full_review_counts_repopilotignore_files_in_the_requested_scope() {
    let temp = prepared_repo();
    fs::write(temp.path().join(".repopilotignore"), "ignored.rs\n").unwrap();
    fs::write(temp.path().join("ignored.rs"), "fn ignored() {}\n").unwrap();

    let summary = scan_path_with_config(temp.path(), &ScanConfig::default()).unwrap();
    let expected_requested =
        summary.metrics.files_discovered + summary.metrics.files_skipped_repopilotignore;
    let report = build_review_report(
        summary,
        temp.path(),
        None,
        None,
        None,
        &RepoPilotConfig::default(),
    )
    .unwrap();
    let readiness = derive_readiness(&report, None, None, None);
    let proof = derive_change_proof_from_review(&report, &readiness);
    let evidence = EvidenceSummary::from_review(&report, &proof);

    assert_eq!(expected_requested, 3);
    assert_eq!(proof.coverage.requested_files, expected_requested);
    assert_eq!(proof.coverage.excluded_files, 1);
    assert_eq!(
        proof.coverage.analyzed_files
            + proof.coverage.excluded_files
            + proof.coverage.unsupported_files
            + proof.coverage.policy_skipped_files,
        proof.coverage.requested_files
    );
    assert!(evidence.coverage_limits.iter().any(|limit| {
        limit.code == "files-repopilotignore"
            && limit.count == 1
            && limit.message == "1 file was excluded by .repopilotignore."
    }));
    assert!(
        !evidence
            .coverage_limits
            .iter()
            .any(|limit| limit.code == "unaccounted-files")
    );
    assert!(
        evidence
            .scope_line()
            .contains("full; 2/3 file(s) analyzed; 1 excluded, 0 unsupported")
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

#[test]
fn intent_drift_is_projected_into_canonical_change_proof_without_hiding_evidence() {
    let temp = prepared_repo();
    fs::write(
        temp.path().join("src/lib.rs"),
        "pub fn answer() -> u8 { 42 }\n",
    )
    .unwrap();
    let mut report = review_report(&temp);
    report.intent.contract = Some(IntentContract {
        version: 1,
        summary: Some("Auth-only change".to_string()),
        paths: vec!["src/auth/**".to_string()],
        contract_families: Vec::new(),
        critical_paths: Vec::new(),
        verification: Vec::new(),
    });
    let readiness = derive_readiness(&report, None, None, None);
    let proof = derive_change_proof_from_review(&report, &readiness);

    assert_eq!(proof.verdict, ChangeProofVerdict::Review);
    assert_eq!(proof.intent_drift.status, IntentStatus::Drifted);
    assert_eq!(proof.intent_drift.unexpected_paths, vec!["src/lib.rs"]);
    assert!(
        proof
            .reasons
            .iter()
            .any(|reason| { reason.code == ChangeProofReasonCode::IntentDrift })
    );
}

#[test]
fn library_review_builder_preserves_configured_critical_paths() {
    let temp = prepared_repo();
    fs::write(
        temp.path().join("src/lib.rs"),
        "pub fn answer() -> u8 { 42 }\n",
    )
    .unwrap();
    let summary = scan_changed_with_config(temp.path(), &ScanConfig::default(), None).unwrap();
    let mut config = RepoPilotConfig::default();
    config.review.critical_paths = vec![CriticalPathRule {
        name: "core".to_string(),
        paths: vec!["src/**".to_string()],
    }];

    let report = build_review_report(summary, temp.path(), None, None, None, &config).unwrap();
    let readiness = derive_readiness(&report, None, None, None);
    let proof = derive_change_proof_from_review(&report, &readiness);

    assert_eq!(report.intent.critical_paths, config.review.critical_paths);
    assert_eq!(proof.intent_drift.status, IntentStatus::NotSupplied);
    assert_eq!(proof.intent_drift.critical_path_matches[0].name, "core");
    assert_eq!(
        proof.intent_drift.critical_path_matches[0].paths,
        ["src/lib.rs"]
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
        diagnostics: None,
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
