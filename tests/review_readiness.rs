use repopilot::baseline::gate::{CiGateResult, FailOn};
use repopilot::findings::provenance::AnalysisScope;
use repopilot::findings::types::Severity;
use repopilot::review::diff::{ChangeStatus, ChangedFile};
use repopilot::review::model::ReviewReport;
use repopilot::review::signals::tiered::{
    ConfidenceTier, ReviewSignal, ReviewSignalProvenance, ReviewSignalVerificationPlan,
    SignalFamily,
};
use repopilot::review::{
    MergeReadinessRecord, OwnershipAssessment, OwnershipSummary, ReadinessReasonCode,
    ReadinessVerdict, derive_readiness,
};
use repopilot::rules::{RuleLifecycle, SignalSource};
use repopilot::scan::types::{ScanMetadata, ScanMetrics, ScanMode, ScanSummary};
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
    let index = repopilot::review::OwnershipIndex::from_codeowners(
        "/src/ @team\n",
        PathBuf::from("CODEOWNERS"),
    )
    .unwrap();
    let ownership = OwnershipSummary::for_paths([PathBuf::from("docs/architecture.md")], &index);
    let readiness = derive_readiness(&report_with_ownership(ownership), None, None, None);

    assert_eq!(readiness.verdict, ReadinessVerdict::Review);
    assert_eq!(
        readiness.ownership.assessment,
        OwnershipAssessment::ConfiguredButUnmatched
    );
    assert!(
        readiness.reasons.iter().any(|reason| {
            reason.code == ReadinessReasonCode::UnownedSurface && reason.count == 1
        })
    );
}

#[test]
fn missing_codeowners_is_disclosed_without_becoming_a_review_reason() {
    let ownership = OwnershipSummary::for_paths(
        [PathBuf::from("src/auth/session.rs")],
        &repopilot::review::OwnershipIndex::empty(),
    );
    let readiness = derive_readiness(&report_with_ownership(ownership), None, None, None);

    assert_eq!(readiness.verdict, ReadinessVerdict::Ready);
    assert_eq!(
        readiness.ownership.assessment,
        OwnershipAssessment::NotConfigured
    );
    assert!(readiness.ownership.unowned_paths.is_empty());
    assert!(
        !readiness
            .reasons
            .iter()
            .any(|reason| { reason.code == ReadinessReasonCode::UnownedSurface })
    );
    assert!(
        readiness
            .limitations
            .iter()
            .any(|item| { item.contains("Ownership is not assessed") })
    );
    let report = report_with_ownership(readiness.ownership.clone());
    let console = repopilot::review::render::render_console(&report, None);
    let markdown = repopilot::review::render::render_markdown(&report, None);
    assert!(console.contains("Ownership: not configured (not assessed)"));
    assert!(markdown.contains("**Ownership:** `not configured (not assessed)`"));
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
    assert_eq!(
        readiness.ownership.assessment,
        OwnershipAssessment::Resolved
    );
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
    assert_eq!(json["change_proof"]["verdict"], "REVIEW");
    assert_eq!(json["change_proof"]["coverage"]["scope"], "changed");
    assert_eq!(json["change_proof"]["coverage"]["analyzed_files"], 1);
    assert_eq!(json["change_proof"]["obligations"]["applicable"], 0);
    assert!(json["change_proof"]["capability_coverage"].is_array());
    assert_eq!(json["merge_readiness"]["impact"]["depth"], 0);
    assert_eq!(
        json["merge_readiness"]["ownership"]["suggested_owners"][0]["value"],
        "@team"
    );
    assert_eq!(
        json["merge_readiness"]["ownership"]["assessment"],
        "resolved"
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
    assert!(console.contains("Legacy merge readiness: READY"));
    assert!(console.contains("Change Proof: REVIEW"));
    assert!(!console.contains("Decision: PASS"));
    assert!(
        console.find("Change Proof: REVIEW").unwrap()
            < console.find("Legacy merge readiness: READY").unwrap()
    );
    assert!(console.contains("Proof scope: 1/1 file(s) analyzed"));
    assert!(console.contains("Evidence class: SUSPICION"));
    assert!(console.contains(
        "Evidence scope: changed; 1/1 file(s) analyzed; 0 excluded, 0 unsupported (complete)"
    ));
    assert!(console.contains("Evidence provenance: RepoPilot 0.22.0, schema 0.26"));
    assert!(console.contains("Proof policy: none selected (0 configured)"));
    assert!(console.contains("Reasons:"));
    assert!(console.contains("Next action: Configure or select a proof policy"));
    assert!(console.contains("Suggested owners: @team"));
    assert!(console.contains("Ownership: resolved"));
    assert!(console.contains("CI gate: not configured"));
    assert!(console.contains("Review gate: not configured"));
    assert!(markdown.contains("**Legacy merge readiness:** `ready`"));
    assert!(markdown.contains("**Change proof:** `REVIEW`"));
    assert!(
        markdown.find("**Change proof:** `REVIEW`").unwrap()
            < markdown
                .find("**Legacy merge readiness:** `ready`")
                .unwrap()
    );
    assert!(markdown.contains("**Proof scope:** 1/1 file(s) analyzed"));
    assert!(markdown.contains("**Evidence class:** `SUSPICION`"));
    assert!(markdown.contains(
        "**Evidence scope:** changed; 1/1 file(s) analyzed; 0 excluded, 0 unsupported (complete)"
    ));
    assert!(markdown.contains("**Evidence provenance:** RepoPilot 0.22.0, schema 0.26"));
    assert!(markdown.contains("**Proof policy:** none selected (0 configured)"));
    assert!(markdown.contains("**Reasons:**"));
    assert!(markdown.contains("**Next action:** Configure or select a proof policy"));
    assert!(markdown.contains("**Ownership:** `resolved`"));
    assert!(markdown.contains("**Suggested owners:** `@team`"));
    assert!(markdown.contains("**CI gate:** not configured"));
    assert!(markdown.contains("**Review gate:** not configured"));
}

#[test]
fn empty_review_is_not_assessed_and_explains_the_missing_scope() {
    let mut report = report_with_ownership(OwnershipSummary::default());
    report.changed_files.clear();
    report.summary.metrics.files_discovered = 0;
    report.summary.metrics.files_analyzed = 0;

    let console = repopilot::review::render::render_console(&report, None);
    let markdown = repopilot::review::render::render_markdown(&report, None);

    assert!(console.contains("Change Proof: NOT ASSESSED"));
    assert!(console.contains("Evidence class: UNKNOWN"));
    assert!(console.contains(
        "Evidence scope: changed; 0/0 file(s) analyzed; 0 excluded, 0 unsupported (unavailable)"
    ));
    assert!(console.contains("Why: No changed files were available for assessment."));
    assert!(console.contains(
        "Next action: Expand the analyzable scope before treating this review as evidence."
    ));
    assert!(console.contains(
        "Legacy merge readiness: READY (COMPATIBILITY FIELD; NO CHANGED SCOPE ASSESSED)"
    ));
    assert!(!console.contains("Decision: PASS"));
    assert!(markdown.contains("**Change proof:** `NOT ASSESSED`"));
    assert!(markdown.contains("**Evidence class:** `UNKNOWN`"));
    assert!(markdown.contains("**Why:** No changed files were available for assessment."));
    assert!(markdown.contains(
        "**Next action:** Expand the analyzable scope before treating this review as evidence."
    ));
    assert!(markdown.contains(
        "**Legacy merge readiness:** `ready (compatibility field; no changed scope assessed)`"
    ));
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
fn human_summary_discloses_when_no_verification_was_selected() {
    let report = report_with_ownership(OwnershipSummary::default());

    let console = repopilot::review::render::render_console(&report, None);
    let markdown = repopilot::review::render::render_markdown(&report, None);

    assert!(console.contains("Verification proof: none selected; no verification evidence"));
    assert!(markdown.contains("- **Verification proof:** none selected; no verification evidence"));
}

#[test]
fn readiness_includes_visible_signal_verification_guidance() {
    let mut report = report_with_ownership(OwnershipSummary::default());
    report.tiered_signals.definitely.push(ReviewSignal {
        signal_id: "signal-1".to_string(),
        kind: "boundary.access-control".to_string(),
        family: SignalFamily::Boundary,
        tier: ConfidenceTier::DefinitelySensitive,
        confidence: repopilot::findings::types::Confidence::High,
        path: "src/auth.rs".to_string(),
        target_path: None,
        line: Some(4),
        line_start: Some(4),
        line_end: Some(4),
        evidence_lines: vec![4],
        headline: "access control changed".to_string(),
        detail: None,
        blast_radius: 0,
        provenance: ReviewSignalProvenance {
            detector: "boundary.access-control".to_string(),
            lifecycle: RuleLifecycle::Preview,
            signal_source: SignalSource::GitDiff,
            analysis_scope: AnalysisScope::GitDiff,
        },
        suppressed: false,
        suppression_reason: None,
        gate_eligible: true,
        verification_plan: Some(ReviewSignalVerificationPlan {
            steps: vec!["confirm access behavior".to_string()],
        }),
    });

    let readiness = derive_readiness(&report, None, None, None);

    assert_eq!(
        readiness.verification_steps,
        vec!["confirm access behavior".to_string()]
    );
    let rendered = repopilot::review::render::render_json(&report, None).unwrap();
    let json: serde_json::Value = serde_json::from_str(&rendered).unwrap();
    assert_eq!(
        json["merge_readiness"]["verification_steps"][0],
        "confirm access behavior"
    );
    assert_eq!(json["change_proof"]["obligations"]["applicable"], 1);
    assert_eq!(json["change_proof"]["obligations"]["unavailable"], 1);
}

#[test]
fn human_summary_reports_revision_compatible_passed_verification() {
    let mut report = report_with_ownership(OwnershipSummary::default());
    report.verification = vec![verification_outcome(VerificationStatus::Passed, true)];

    let console = repopilot::review::render::render_console(&report, None);
    let markdown = repopilot::review::render::render_markdown(&report, None);

    let summary = "1 passed, 0 failed, 0 unavailable, 0 unselected, 0 stale (revision-compatible)";
    assert!(console.contains(&format!("Verification proof: {summary}")));
    assert!(markdown.contains(&format!("- **Verification proof:** {summary}")));
}

#[test]
fn human_summary_reports_failed_verification() {
    let mut report = report_with_ownership(OwnershipSummary::default());
    report.verification = vec![verification_outcome(VerificationStatus::Failed, true)];

    let console = repopilot::review::render::render_console(&report, None);
    let markdown = repopilot::review::render::render_markdown(&report, None);

    let summary = "0 passed, 1 failed, 0 unavailable, 0 unselected, 0 stale (revision-compatible)";
    assert!(console.contains(&format!("Verification proof: {summary}")));
    assert!(markdown.contains(&format!("- **Verification proof:** {summary}")));
}

#[test]
fn human_summary_reports_unavailable_verification() {
    let mut report = report_with_ownership(OwnershipSummary::default());
    report.verification = vec![verification_outcome(VerificationStatus::Unavailable, true)];

    let console = repopilot::review::render::render_console(&report, None);
    let markdown = repopilot::review::render::render_markdown(&report, None);

    let summary = "0 passed, 0 failed, 1 unavailable, 0 unselected, 0 stale (revision-compatible)";
    assert!(console.contains(&format!("Verification proof: {summary}")));
    assert!(markdown.contains(&format!("- **Verification proof:** {summary}")));
}

#[test]
fn human_summary_reports_revision_incompatible_verification() {
    let mut report = report_with_ownership(OwnershipSummary::default());
    report.verification = vec![verification_outcome(VerificationStatus::Passed, false)];

    let console = repopilot::review::render::render_console(&report, None);
    let markdown = repopilot::review::render::render_markdown(&report, None);

    let summary =
        "0 passed, 0 failed, 0 unavailable, 0 unselected, 1 stale (revision-incompatible)";
    assert!(console.contains(&format!("Verification proof: {summary}")));
    assert!(markdown.contains(&format!("- **Verification proof:** {summary}")));
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
        analysis_revision: None,
        summary: ScanSummary {
            metadata: ScanMetadata {
                mode: ScanMode::Changed,
                ..Default::default()
            },
            metrics: ScanMetrics {
                files_discovered: 1,
                files_analyzed: 1,
                ..Default::default()
            },
            ..Default::default()
        },
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
        verification_policy: Default::default(),
        verification: Vec::new(),
        intent: Default::default(),
        findings: Vec::new(),
    }
}

fn _assert_serializable(_: &MergeReadinessRecord) {}
