use super::{render_review_sarif, severity_for_tier};
use crate::findings::provenance::AnalysisScope;
use crate::findings::types::{Confidence, Severity};
use crate::review::model::ReviewReport;
use crate::review::signals::tiered::{
    ConfidenceTier, ReviewSignal, ReviewSignalProvenance, SignalFamily, TieredSignals,
};
use crate::rules::{RuleLifecycle, SignalSource};
use crate::scan::types::ScanSummary;
use std::path::Path;

fn taint_signal(kind: &str, tier: ConfidenceTier) -> ReviewSignal {
    ReviewSignal {
        signal_id: format!("{kind}:src/app.py:1"),
        kind: kind.to_string(),
        family: SignalFamily::Taint,
        tier,
        confidence: Confidence::High,
        path: "src/app.py".to_string(),
        target_path: None,
        line: Some(1),
        line_start: Some(1),
        line_end: Some(1),
        evidence_lines: Vec::new(),
        headline: "untrusted input reaches a sink".to_string(),
        detail: None,
        blast_radius: 0,
        provenance: ReviewSignalProvenance {
            detector: "taint".to_string(),
            lifecycle: RuleLifecycle::Stable,
            signal_source: SignalSource::Ast,
            analysis_scope: AnalysisScope::GitDiff,
        },
        suppressed: false,
        suppression_reason: None,
        gate_eligible: true,
        verification_plan: None,
    }
}

fn report_with_signal(signal: ReviewSignal) -> ReviewReport {
    let mut tiered_signals = TieredSignals::default();
    match signal.tier {
        ConfidenceTier::DefinitelySensitive => tiered_signals.definitely.push(signal),
        ConfidenceTier::MaybeSensitive => tiered_signals.maybe.push(signal),
        ConfidenceTier::LargeDiffOrNoise => tiered_signals.noise.push(signal),
    }
    ReviewReport {
        summary: ScanSummary::default(),
        repo_root: Path::new("/repo").to_path_buf(),
        baseline_path: None,
        changed_files: Vec::new(),
        blast_radius: Vec::new(),
        impact_paths: Default::default(),
        ownership: Default::default(),
        ownership_diagnostics: Vec::new(),
        boundary_signals: Vec::new(),
        boundary_missing_test: false,
        tiered_signals,
        timings: Default::default(),
        verification: Vec::new(),
        findings: Vec::new(),
    }
}

#[test]
fn severity_is_read_from_the_tier_not_the_kind_string() {
    // The regression this pins: an unrecognized `kind` string used to fall
    // through to Medium regardless of its actual tier. A signal whose kind is
    // neither "taint.sql" nor "taint.exec" but is still tiered
    // DefinitelySensitive must render as High, proving severity now tracks
    // `SinkKind`'s canonical tier rather than a second, separately maintained
    // string match.
    assert_eq!(
        severity_for_tier(ConfidenceTier::DefinitelySensitive),
        Severity::High
    );
    assert_eq!(
        severity_for_tier(ConfidenceTier::MaybeSensitive),
        Severity::Medium
    );

    let report = report_with_signal(taint_signal(
        "taint.deserialize",
        ConfidenceTier::DefinitelySensitive,
    ));
    let rendered = render_review_sarif(&report).expect("sarif renders");
    let value: serde_json::Value = serde_json::from_str(&rendered).expect("valid json");
    let level = value["runs"][0]["results"][0]["level"]
        .as_str()
        .expect("result level");
    assert_eq!(
        level, "error",
        "DefinitelySensitive must map to SARIF error (High)"
    );
}
