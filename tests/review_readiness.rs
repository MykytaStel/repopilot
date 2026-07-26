use repopilot::baseline::gate::{CiGateResult, FailOn};
use repopilot::findings::types::Severity;
use repopilot::review::diff::{ChangeStatus, ChangedFile};
use repopilot::review::model::ReviewReport;
use repopilot::review::{
    MergeReadinessRecord, OwnershipSummary, ReadinessReasonCode, ReadinessVerdict, derive_readiness,
};
use repopilot::scan::types::ScanSummary;
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
        findings: Vec::new(),
    }
}

fn _assert_serializable(_: &MergeReadinessRecord) {}
