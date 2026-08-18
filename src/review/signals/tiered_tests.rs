use super::algorithmic::{AlgorithmicKind, AlgorithmicSignal};
use super::api_contract::{RemovedExportSignal, SymbolKind};
use super::behavioral::{BehavioralKind, BehavioralSignal, BehavioralSignalSource};
use super::taint::{SinkKind, SourceKind, TaintSignal};
use super::tiered::{ConfidenceTier, SignalFamily, build_tiered, build_tiered_with_api_contract};
use super::{BoundaryCategory, BoundarySignal};
use crate::findings::types::Confidence;
use crate::review::diff::{ChangeStatus, ChangedFile, ChangedRange};
use crate::scan::types::CouplingGraph;
use std::path::Path;
use std::path::PathBuf;

fn boundary(category: BoundaryCategory, path: &str) -> BoundarySignal {
    BoundarySignal {
        category,
        path: path.to_string(),
        status: ChangeStatus::Modified,
        blast_radius: 0,
    }
}

fn behavioral(kind: BehavioralKind, path: &str) -> BehavioralSignal {
    BehavioralSignal {
        kind,
        path: path.to_string(),
        line: 1,
        detail: "detail".to_string(),
        source: BehavioralSignalSource::Ast,
    }
}

fn algorithmic(kind: AlgorithmicKind, path: &str) -> AlgorithmicSignal {
    AlgorithmicSignal {
        kind,
        path: path.to_string(),
        line: 1,
        detail: "detail".to_string(),
    }
}

fn taint(sink: SinkKind, path: &str) -> TaintSignal {
    TaintSignal {
        source: SourceKind::HttpRequest,
        sink,
        path: path.to_string(),
        line: 1,
        detail: "detail".to_string(),
    }
}

fn removed_export(
    exporter: &str,
    importer: &str,
    exported_name: &str,
    local_name: &str,
    line_start: usize,
    line_end: usize,
) -> RemovedExportSignal {
    RemovedExportSignal {
        exporter_path: PathBuf::from(exporter),
        importer_path: PathBuf::from(importer),
        exported_name: exported_name.to_string(),
        local_name: local_name.to_string(),
        symbol_kind: SymbolKind::Value,
        module_specifier: "./api.ts".to_string(),
        line_start,
        line_end,
        byte_start: line_start * 100,
        byte_end: line_end * 100 + 10,
    }
}

/// A large diff with `count` files of 40 changed lines each (no signals).
fn large_diff(count: usize) -> Vec<ChangedFile> {
    (0..count)
        .map(|index| ChangedFile {
            path: PathBuf::from(format!("src/f{index}.rs")),
            status: ChangeStatus::Modified,
            ranges: vec![ChangedRange { start: 1, end: 40 }],
            hunks: Vec::new(),
        })
        .collect()
}

#[test]
fn boundary_signals_land_in_definitely() {
    let tiered = build_tiered(
        &[boundary(BoundaryCategory::AccessControl, "src/auth.ts")],
        &[],
        &[],
        &[],
        &[],
    );
    assert_eq!(tiered.definitely.len(), 1);
    assert!(tiered.maybe.is_empty());
    assert!(tiered.noise.is_empty());
}

#[test]
fn behavioral_kinds_tier_by_sensitivity() {
    let signals = [
        behavioral(BehavioralKind::EnvVarIntroduced, "src/config.ts"),
        behavioral(BehavioralKind::NetworkCallAdded, "src/api.ts"),
    ];
    let tiered = build_tiered(&[], &signals, &[], &[], &[]);
    // env var is a definite boundary crossing; a plain network call is maybe.
    assert_eq!(tiered.definitely.len(), 1);
    assert_eq!(tiered.maybe.len(), 1);
}

#[test]
fn coarse_behavioral_signals_are_demoted_to_noise() {
    // An AuthCheckRemoved from the coarse (non-AST) fallback would normally be
    // "definitely sensitive"; its CoarseFallback source drops it to the noise tier.
    let coarse = BehavioralSignal {
        kind: BehavioralKind::AuthCheckRemoved,
        path: "src/legacy.php".to_string(),
        line: 1,
        detail: "Authentication/authorization check removed (coarse fallback)".to_string(),
        source: BehavioralSignalSource::CoarseFallback,
    };
    let tiered = build_tiered(&[], &[coarse], &[], &[], &[]);
    assert!(tiered.definitely.is_empty());
    assert!(tiered.maybe.is_empty());
    assert_eq!(tiered.noise.len(), 1);
    assert_eq!(tiered.noise[0].tier, ConfidenceTier::LargeDiffOrNoise);
}

#[test]
fn same_kind_is_definitely_when_ast_sourced() {
    // The identical kind, but AST-sourced, stays in the definitely tier — proving
    // the tier keys off `source`, not the kind or the detail text.
    let ast = BehavioralSignal {
        kind: BehavioralKind::AuthCheckRemoved,
        path: "src/auth.ts".to_string(),
        line: 1,
        detail: "Authentication/authorization check removed (coarse fallback)".to_string(),
        source: BehavioralSignalSource::Ast,
    };
    let tiered = build_tiered(&[], &[ast], &[], &[], &[]);
    assert_eq!(tiered.definitely.len(), 1);
    assert!(tiered.noise.is_empty());
}

#[test]
fn network_call_in_an_auth_path_is_escalated() {
    let tiered = build_tiered(
        &[boundary(BoundaryCategory::AccessControl, "src/auth.ts")],
        &[behavioral(BehavioralKind::NetworkCallAdded, "src/auth.ts")],
        &[],
        &[],
        &[],
    );
    // The boundary itself plus the network call in that auth file are both definite.
    assert_eq!(tiered.definitely.len(), 2);
    assert!(tiered.maybe.is_empty());
}

#[test]
fn algorithmic_signals_are_maybe() {
    let tiered = build_tiered(
        &[],
        &[],
        &[algorithmic(
            AlgorithmicKind::NestedLoopIntroduced,
            "src/x.rs",
        )],
        &[],
        &[],
    );
    assert_eq!(tiered.maybe.len(), 1);
    assert!(tiered.definitely.is_empty());
}

#[test]
fn taint_tiers_by_sink_severity() {
    // SQL/exec injection is top tier; filesystem/network reach is maybe.
    let tiered = build_tiered(
        &[],
        &[],
        &[],
        &[
            taint(SinkKind::Sql, "src/db.ts"),
            taint(SinkKind::FsWrite, "src/files.ts"),
        ],
        &[],
    );
    assert_eq!(tiered.definitely.len(), 1);
    assert_eq!(tiered.maybe.len(), 1);
}

#[test]
fn has_taint_signal_detects_taint_family_only() {
    let with_taint = build_tiered(&[], &[], &[], &[taint(SinkKind::Sql, "src/db.ts")], &[]);
    assert!(with_taint.has_taint_signal());

    let without_taint = build_tiered(
        &[boundary(BoundaryCategory::AccessControl, "src/auth.ts")],
        &[behavioral(BehavioralKind::FsWriteAdded, "src/f0.rs")],
        &[],
        &[],
        &[],
    );
    assert!(!without_taint.has_taint_signal());
}

#[test]
fn noise_tier_fires_only_for_a_large_diff_with_nothing_flagged() {
    let tiered = build_tiered(&[], &[], &[], &[], &large_diff(6));
    assert_eq!(tiered.noise.len(), 1);
    assert_eq!(tiered.noise[0].tier, ConfidenceTier::LargeDiffOrNoise);
}

#[test]
fn noise_tier_is_suppressed_when_a_signal_is_present() {
    let tiered = build_tiered(
        &[],
        &[behavioral(BehavioralKind::FsWriteAdded, "src/f0.rs")],
        &[],
        &[],
        &large_diff(6),
    );
    assert!(tiered.noise.is_empty());
    assert_eq!(tiered.maybe.len(), 1);
}

#[test]
fn small_diff_with_nothing_flagged_is_silent() {
    let tiered = build_tiered(&[], &[], &[], &[], &large_diff(2));
    assert!(tiered.is_empty());
}

#[test]
fn definitely_sensitive_signals_include_verification_plan() {
    let tiered = build_tiered(
        &[boundary(BoundaryCategory::AccessControl, "src/auth.ts")],
        &[],
        &[],
        &[],
        &[],
    );

    let plan = tiered.definitely[0]
        .verification_plan
        .as_ref()
        .expect("definitely sensitive review signals should include a verification plan");

    assert!(plan.steps.len() >= 3);
    assert!(plan.steps[0].contains("src/auth.ts"));
    assert!(
        plan.steps
            .iter()
            .any(|step| step.contains("static review evidence only"))
    );
}

#[test]
fn maybe_and_noise_signals_do_not_get_verification_plans() {
    let maybe = build_tiered(
        &[],
        &[behavioral(BehavioralKind::NetworkCallAdded, "src/api.ts")],
        &[],
        &[],
        &[],
    );
    assert!(maybe.definitely.is_empty());
    assert!(maybe.maybe[0].verification_plan.is_none());

    let noise = build_tiered(&[], &[], &[], &[], &large_diff(6));
    assert!(noise.noise[0].verification_plan.is_none());
}

#[test]
fn review_signal_verification_plans_are_deterministic() {
    let left = build_tiered(
        &[],
        &[behavioral(
            BehavioralKind::EnvVarIntroduced,
            "src/config.ts",
        )],
        &[],
        &[],
        &[],
    );
    let right = build_tiered(
        &[],
        &[behavioral(
            BehavioralKind::EnvVarIntroduced,
            "src/config.ts",
        )],
        &[],
        &[],
        &[],
    );

    assert_eq!(
        left.definitely[0].verification_plan,
        right.definitely[0].verification_plan
    );
}

#[test]
fn review_signal_verification_plan_serializes_as_stable_steps_array() {
    let tiered = build_tiered(
        &[boundary(BoundaryCategory::AccessControl, "src/auth.ts")],
        &[],
        &[],
        &[],
        &[],
    );

    let value = serde_json::to_value(&tiered.definitely[0])
        .expect("review signals should serialize to JSON");
    let steps = value
        .get("verification_plan")
        .and_then(|plan| plan.get("steps"))
        .and_then(serde_json::Value::as_array)
        .expect("verification_plan.steps should be a stable JSON array");

    assert_eq!(steps.len(), 3);
    assert_eq!(
        steps[0].as_str(),
        Some(
            "Open src/auth.ts and confirm the changed diff still supports the review signal: access control changed."
        )
    );
    assert!(steps.iter().any(|step| {
        step.as_str()
            .is_some_and(|step| step.contains("static review evidence only"))
    }));
}

#[test]
fn maybe_signal_omits_verification_plan_in_json_contract() {
    let tiered = build_tiered(
        &[],
        &[behavioral(BehavioralKind::NetworkCallAdded, "src/api.ts")],
        &[],
        &[],
        &[],
    );

    let value =
        serde_json::to_value(&tiered.maybe[0]).expect("review signals should serialize to JSON");

    assert!(value.get("verification_plan").is_none());
    assert!(value.get("target_path").is_none());
}

#[test]
fn removed_export_occurrences_keep_distinct_stable_ids() {
    // Catches collapsing two removed symbols in one caller or making identity
    // depend on detector input order instead of the full import occurrence.
    let occurrences = vec![
        removed_export("src/api.ts", "src/caller.ts", "loadUser", "load", 10, 10),
        removed_export("src/api.ts", "src/caller.ts", "saveUser", "save", 12, 12),
    ];
    let forward = build_tiered_with_api_contract(&[], &[], &[], &[], &occurrences, &[]);
    let reversed = build_tiered_with_api_contract(
        &[],
        &[],
        &[],
        &[],
        &occurrences.iter().cloned().rev().collect::<Vec<_>>(),
        &[],
    );

    assert_eq!(forward.definitely.len(), 2);
    assert!(forward.maybe.is_empty());
    assert!(forward.noise.is_empty());
    assert_eq!(forward.definitely, reversed.definitely);
    assert_eq!(
        forward
            .definitely
            .iter()
            .map(|signal| signal.signal_id.as_str())
            .collect::<Vec<_>>(),
        vec!["1a407d181dc61405", "85f5db2e7b995998"]
    );

    let signal = &forward.definitely[0];
    assert_eq!(signal.kind, "behavioral.removed-export-still-imported");
    assert_eq!(signal.family, SignalFamily::Behavioral);
    assert_eq!(signal.tier, ConfidenceTier::DefinitelySensitive);
    assert_eq!(signal.confidence, Confidence::High);
    assert_eq!(signal.path, "src/caller.ts");
    assert_eq!(signal.target_path.as_deref(), Some("src/api.ts"));
    assert_eq!((signal.line_start, signal.line_end), (Some(10), Some(10)));
    assert_eq!(signal.evidence_lines, vec![10]);
    assert_eq!(signal.headline, "removed export is still imported");
    assert_eq!(
        signal.detail.as_deref(),
        Some(
            "Removed value export 'loadUser' from src/api.ts remains imported as local binding 'load' via './api.ts'."
        )
    );
    assert!(signal.gate_eligible);
    assert!(signal.verification_plan.is_some());
    assert_eq!(
        serde_json::to_value(signal)
            .expect("removed-export signal should serialize")
            .get("target_path")
            .and_then(serde_json::Value::as_str),
        Some("src/api.ts")
    );
}

#[test]
fn same_symbol_same_line_occurrences_use_exact_spans_for_identity() {
    let mut first = removed_export("src/api.ts", "src/caller.ts", "loadUser", "first", 1, 1);
    first.byte_start = 9;
    first.byte_end = 26;
    let mut second = removed_export("src/api.ts", "src/caller.ts", "loadUser", "second", 1, 1);
    second.byte_start = 28;
    second.byte_end = 46;

    let tiered = build_tiered_with_api_contract(&[], &[], &[], &[], &[first, second], &[]);
    assert_eq!(tiered.definitely.len(), 2);
    assert_ne!(
        tiered.definitely[0].signal_id,
        tiered.definitely[1].signal_id
    );
    assert_eq!(tiered.definitely[0].line_start, Some(1));
    assert_eq!(tiered.definitely[1].line_start, Some(1));
}

#[test]
fn removed_export_impact_uses_exporter_target() {
    // Catches impact enrichment looking up the caller evidence path instead of
    // the changed exporter whose public contract lost the symbol.
    let occurrence = removed_export(
        "src/api.ts",
        "src/caller.ts",
        "loadUser",
        "loadUser",
        10,
        10,
    );
    let mut tiered = build_tiered_with_api_contract(&[], &[], &[], &[], &[occurrence], &[]);
    let mut graph = CouplingGraph::default();
    graph
        .edges
        .entry(PathBuf::from("src/caller.ts"))
        .or_default()
        .insert(PathBuf::from("src/api.ts"));
    graph
        .edges
        .entry(PathBuf::from("src/other.ts"))
        .or_default()
        .insert(PathBuf::from("src/api.ts"));

    super::tiered::enrich_blast_radius(&mut tiered, Some(&graph), Path::new(""));

    assert_eq!(tiered.definitely[0].blast_radius, 2);
}

#[test]
fn removed_export_callers_keep_distinct_stable_ids() {
    // Catches collapsing the same removed symbol when two callers retain their
    // own import occurrences, including a multi-line import span.
    let occurrences = vec![
        removed_export(
            "src/api.ts",
            "src/caller.ts",
            "loadUser",
            "loadUser",
            10,
            10,
        ),
        removed_export("src/api.ts", "src/other.ts", "loadUser", "load", 4, 5),
    ];
    let forward = build_tiered_with_api_contract(&[], &[], &[], &[], &occurrences, &[]);
    let reversed = build_tiered_with_api_contract(
        &[],
        &[],
        &[],
        &[],
        &occurrences.iter().cloned().rev().collect::<Vec<_>>(),
        &[],
    );

    assert_eq!(forward.definitely, reversed.definitely);
    assert_eq!(forward.definitely.len(), 2);
    assert_eq!(
        forward
            .definitely
            .iter()
            .map(|signal| signal.signal_id.as_str())
            .collect::<Vec<_>>(),
        vec!["1a407d181dc61405", "d1218d934e09bf27"]
    );
    assert_eq!(
        forward.definitely[1].path, "src/other.ts",
        "caller evidence must remain occurrence-specific"
    );
    assert_eq!(
        (
            forward.definitely[1].line_start,
            forward.definitely[1].line_end
        ),
        (Some(4), Some(5))
    );
}
