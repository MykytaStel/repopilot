use repopilot::findings::provenance::AnalysisScope;
use repopilot::findings::types::Confidence;
use repopilot::output::decision_summary::review_decision_summary;
use repopilot::review::diff::{ChangeStatus, ChangedFile};
use repopilot::review::model::ReviewReport;
use repopilot::review::signals::tiered::{
    ConfidenceTier, ReviewSignal, ReviewSignalProvenance, ReviewSignalVerificationPlan,
    SignalFamily, TieredSignals,
};
use repopilot::rules::{RuleLifecycle, SignalSource};
use repopilot::scan::types::{ScanMode, ScanSummary};
use std::path::PathBuf;

#[test]
fn definitely_sensitive_signal_plan_counts_as_decision_input() {
    let mut summary = ScanSummary::default();
    summary.metadata.mode = ScanMode::Changed;
    summary.metrics.files_analyzed = 1;
    let report = ReviewReport {
        summary,
        repo_root: PathBuf::from("/repo"),
        baseline_path: None,
        changed_files: vec![ChangedFile {
            path: PathBuf::from("src/auth.rs"),
            status: ChangeStatus::Modified,
            ranges: Vec::new(),
            hunks: Vec::new(),
        }],
        blast_radius: Vec::new(),
        impact_paths: Default::default(),
        ownership: Default::default(),
        ownership_diagnostics: Vec::new(),
        boundary_signals: Vec::new(),
        boundary_missing_test: false,
        tiered_signals: TieredSignals {
            definitely: vec![ReviewSignal {
                signal_id: "signal-1".to_string(),
                kind: "boundary.access-control".to_string(),
                family: SignalFamily::Boundary,
                tier: ConfidenceTier::DefinitelySensitive,
                confidence: Confidence::High,
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
            }],
            maybe: Vec::new(),
            noise: Vec::new(),
        },
        timings: Default::default(),
        verification_policy: Default::default(),
        verification: Vec::new(),
        findings: Vec::new(),
    };

    let decision = review_decision_summary(&report, None, None);

    assert_eq!(decision.verification_plans, 1);
}
