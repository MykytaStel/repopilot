use repopilot::baseline::gate::{CiGateResult, FailOn};
use repopilot::findings::types::Severity;
use repopilot::review::diff::{ChangeStatus, ChangedFile};
use repopilot::review::model::ReviewReport;
use repopilot::review::{
    MergeReadinessRecord, OwnershipSummary, ReadinessReasonCode, ReadinessVerdict, derive_readiness,
};
use repopilot::scan::types::ScanSummary;
use repopilot::verification::{VerificationOutcome, VerificationRole, VerificationStatus};
use std::path::PathBuf;

#[test]
fn failed_finding_gate_is_blocked_with_stable_reason_code() {
    let report = report_with_ownership(OwnershipSummary::default());
    let gate = CiGateResult {
        fail_on: FailOn::Any(Severity::High),
        failed_findings: 1,
    };

    let readiness = derive_readiness(&report, Some(&gate), None, None);
    assert_eq!(readiness.verdict, ReadinessVerdict::Blocked);
    assert_eq!(
        readiness.reasons[0].code,
        ReadinessReasonCode::FindingGateFailed
    );
}

#[test]
fn unowned_changed_surface_requires_review() {
    let ownership = OwnershipSummary::for_paths(
        [PathBuf::from("src/auth/session.rs")],
        &repopilot::review::OwnershipIndex::empty(),
    );
    let readiness = derive_readiness(&report_with_ownership(ownership), None, None, None);

    assert_eq!(readiness.verdict, ReadinessVerdict::Review);
    assert!(
        readiness.reasons.iter().any(|reason| {
            reason.code == ReadinessReasonCode::UnownedSurface && reason.count == 1
        })
    );
}

#[test]
fn clean_owned_change_is_ready() {
    let index = repopilot::review::OwnershipIndex::from_codeowners(
        "/src/ @team\n",
        PathBuf::from("CODEOWNERS"),
    )
    .unwrap();
    let ownership = OwnershipSummary::for_paths([PathBuf::from("src/lib.rs")], &index);
    let readiness = derive_readiness(&report_with_ownership(ownership), None, None, None);

    assert_eq!(readiness.verdict, ReadinessVerdict::Ready);
    assert!(readiness.reasons.is_empty());
}

#[test]
fn review_json_projects_the_canonical_readiness_record() {
    let index = repopilot::review::OwnershipIndex::from_codeowners(
        "/src/ @team\n",
        PathBuf::from("CODEOWNERS"),
    )
    .unwrap();
    let report = report_with_ownership(OwnershipSummary::for_paths(
        [PathBuf::from("src/lib.rs")],
        &index,
    ));
    let rendered = repopilot::review::render::render_json(&report, None).unwrap();
    let json: serde_json::Value = serde_json::from_str(&rendered).unwrap();

    assert_eq!(json["merge_readiness"]["verdict"], "ready");
    assert_eq!(json["merge_readiness"]["impact"]["depth"], 0);
    assert_eq!(
        json["merge_readiness"]["ownership"]["suggested_owners"][0]["value"],
        "@team"
    );
    assert!(json["merge_readiness"]["limitations"].is_array());
}

#[test]
fn human_reports_project_readiness_and_owners() {
    let index = repopilot::review::OwnershipIndex::from_codeowners(
        "/src/ @team\n",
        PathBuf::from("CODEOWNERS"),
    )
    .unwrap();
    let report = report_with_ownership(OwnershipSummary::for_paths(
        [PathBuf::from("src/lib.rs")],
        &index,
    ));

    let console = repopilot::review::render::render_console(&report, None);
    let markdown = repopilot::review::render::render_markdown(&report, None);
    assert!(console.contains("Merge readiness: READY"));
    assert!(console.contains("Suggested owners: @team"));
    assert!(markdown.contains("**Merge readiness:** `ready`"));
    assert!(markdown.contains("**Suggested owners:** `@team`"));
}

#[test]
fn human_reports_show_when_verification_was_reused() {
    let mut report = report_with_ownership(OwnershipSummary::default());
    report.verification = vec![verification_outcome(VerificationStatus::Passed, true)];

    let executed_console = repopilot::review::render::render_console(&report, None);
    let executed_markdown = repopilot::review::render::render_markdown(&report, None);
    assert!(executed_console.contains("unit: PASSED (10 ms)"));
    assert!(!executed_console.contains("cached"));
    assert!(executed_markdown.contains("| `unit` | `Passed` | executed | 10 ms | 1 |"));

    let mut outcome = report.verification.remove(0);
    outcome.reused = true;
    report.verification = vec![outcome];

    let console = repopilot::review::render::render_console(&report, None);
    let markdown = repopilot::review::render::render_markdown(&report, None);

    assert!(console.contains("unit: PASSED (cached; original run 10 ms)"));
    assert!(!console.contains("unit: PASSED (10 ms, cached)"));
    assert!(markdown.contains("| Check | Status | Source | Duration evidence | Exit |"));
    assert!(markdown.contains("| `unit` | `Passed` | cached | original run 10 ms | 1 |"));
}

#[test]
fn failed_verification_blocks_canonical_readiness() {
    let mut report = report_with_ownership(OwnershipSummary::default());
    report.verification = vec![verification_outcome(VerificationStatus::Failed, true)];

    let readiness = derive_readiness(&report, None, None, None);

    assert_eq!(readiness.verdict, ReadinessVerdict::Blocked);
    assert!(readiness.reasons.iter().any(|reason| {
        reason.code == ReadinessReasonCode::VerificationFailed && reason.count == 1
    }));
    assert_eq!(readiness.verification, report.verification);
}

#[test]
fn revision_incompatible_pass_blocks_readiness() {
    let mut report = report_with_ownership(OwnershipSummary::default());
    report.verification = vec![verification_outcome(VerificationStatus::Passed, false)];

    let readiness = derive_readiness(&report, None, None, None);

    assert_eq!(readiness.verdict, ReadinessVerdict::Blocked);
    assert!(
        readiness
            .reasons
            .iter()
            .any(|reason| { reason.code == ReadinessReasonCode::VerificationRevisionChanged })
    );
}

fn verification_outcome(
    status: VerificationStatus,
    revision_compatible: bool,
) -> VerificationOutcome {
    VerificationOutcome {
        check_id: "unit".to_string(),
        role: VerificationRole::Test,
        status,
        duration_ms: 10,
        exit_code: Some(1),
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

fn report_with_ownership(ownership: OwnershipSummary) -> ReviewReport {
    ReviewReport {
        summary: ScanSummary::default(),
        repo_root: PathBuf::from("/repo"),
        baseline_path: None,
        changed_files: vec![ChangedFile {
            path: PathBuf::from("src/lib.rs"),
            status: ChangeStatus::Modified,
            ranges: Vec::new(),
            hunks: Vec::new(),
        }],
        blast_radius: Vec::new(),
        impact_paths: Default::default(),
        ownership,
        ownership_diagnostics: Vec::new(),
        boundary_signals: Vec::new(),
        boundary_missing_test: false,
        tiered_signals: Default::default(),
        timings: Default::default(),
        verification: Vec::new(),
        findings: Vec::new(),
    }
}

fn _assert_serializable(_: &MergeReadinessRecord) {}
