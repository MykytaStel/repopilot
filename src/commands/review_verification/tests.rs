use super::{ReviewVerificationEvent, run_selected_with_context};
use repopilot::config::loader::parse_config;
use repopilot::findings::visibility::FindingVisibilityProfile;
use repopilot::review::diff::OwnedDiffTarget;
use repopilot::review::model::ReviewReport;
use repopilot::scan::config::ScanConfig;
use repopilot::scan::session::AnalysisSession;
use repopilot::scan::types::ScanSummary;
use repopilot::verification::{CancellationToken, VerificationStatus};
use tempfile::tempdir;

#[test]
fn context_adapter_attaches_outcome_and_emits_safe_lifecycle() {
    let temp = tempdir().expect("temp dir");
    let config = parse_config(
        r#"[[verification.checks]]
id = "unit"
role = "test"
program = "repopilot-missing-unit-program"
"#,
        None,
    )
    .expect("valid config");
    let session = AnalysisSession::new(
        temp.path().to_path_buf(),
        config,
        ScanConfig::default(),
        FindingVisibilityProfile::Default,
    );
    let mut report = empty_report(temp.path());
    let mut events = Vec::new();

    run_selected_with_context(
        &["unit".into()],
        &session,
        &OwnedDiffTarget::WorkingTree,
        &mut report,
        &CancellationToken::new(),
        &mut |event| events.push(event),
    )
    .expect("verification outcome");

    assert_eq!(report.verification.len(), 1);
    assert_eq!(
        report.verification[0].status,
        VerificationStatus::Unavailable
    );
    assert_eq!(
        events,
        vec![
            ReviewVerificationEvent::Started {
                check_id: "unit".into(),
                index: 1,
                total: 1,
            },
            ReviewVerificationEvent::Completed {
                check_id: "unit".into(),
                index: 1,
                total: 1,
                status: VerificationStatus::Unavailable,
            },
        ]
    );
}

#[test]
fn empty_selection_does_not_touch_report_or_observer() {
    let temp = tempdir().expect("temp dir");
    let session = AnalysisSession::new(
        temp.path().to_path_buf(),
        Default::default(),
        ScanConfig::default(),
        FindingVisibilityProfile::Default,
    );
    let mut report = empty_report(temp.path());
    let mut events = Vec::new();

    run_selected_with_context(
        &[],
        &session,
        &OwnedDiffTarget::WorkingTree,
        &mut report,
        &CancellationToken::new(),
        &mut |event| events.push(event),
    )
    .expect("empty selection");

    assert!(report.verification.is_empty());
    assert!(events.is_empty());
}

fn empty_report(root: &std::path::Path) -> ReviewReport {
    ReviewReport {
        summary: ScanSummary::default(),
        repo_root: root.to_path_buf(),
        baseline_path: None,
        changed_files: Vec::new(),
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
