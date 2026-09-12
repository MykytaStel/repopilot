use super::{render_review_sarif, render_review_sarif_with_gates, severity_for_tier};
use crate::findings::provenance::AnalysisScope;
use crate::findings::types::{Confidence, Severity};
use crate::review::diff::{ChangeStatus, ChangedFile};
use crate::review::model::ReviewReport;
use crate::review::signals::tiered::{
    ConfidenceTier, ReviewSignal, ReviewSignalProvenance, SignalFamily, TieredSignals,
};
use crate::review::{ReviewSignalGatePolicy, ReviewSignalGateResult};
use crate::rules::{RuleLifecycle, SignalSource};
use crate::scan::types::ScanSummary;
use std::path::{Path, PathBuf};

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
        verification_policy: Default::default(),
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

#[test]
fn review_sarif_carries_the_canonical_change_proof() {
    let report = report_with_signal(taint_signal(
        "taint.deserialize",
        ConfidenceTier::DefinitelySensitive,
    ));
    let rendered = render_review_sarif(&report).expect("sarif renders");
    let value: serde_json::Value = serde_json::from_str(&rendered).expect("valid json");
    let proof = &value["runs"][0]["properties"]["changeProof"];

    assert!(proof.is_object());
    assert!(proof["verdict"].is_string());
    assert!(proof["reasons"].is_array());
    assert!(proof["coverage"].is_object());
    assert!(proof["obligations"].is_object());
    assert!(proof["contract_deltas"].is_array());
    assert!(proof["capability_coverage"].is_array());
}

#[test]
fn review_sarif_proof_uses_the_review_gate() {
    let mut report = report_with_signal(taint_signal(
        "taint.deserialize",
        ConfidenceTier::DefinitelySensitive,
    ));
    report.summary.metadata.mode = crate::scan::types::ScanMode::Changed;
    report.summary.metrics.files_discovered = 1;
    report.summary.metrics.files_analyzed = 1;
    report.changed_files = vec![ChangedFile {
        path: PathBuf::from("src/app.py"),
        status: ChangeStatus::Modified,
        ranges: Vec::new(),
        hunks: Vec::new(),
    }];
    let review_gate = ReviewSignalGateResult {
        policy: ReviewSignalGatePolicy::Definitely,
        failed_signals: 1,
    };

    let rendered =
        render_review_sarif_with_gates(&report, None, Some(&review_gate)).expect("sarif renders");
    let value: serde_json::Value = serde_json::from_str(&rendered).expect("valid json");
    let proof = &value["runs"][0]["properties"]["changeProof"];

    assert_eq!(proof["verdict"], "REVIEW");
    assert!(proof["reasons"].as_array().is_some_and(|reasons| {
        reasons
            .iter()
            .any(|reason| reason["code"] == "review-signal-gate-failed")
    }));
}
