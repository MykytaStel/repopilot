use super::algorithmic::{AlgorithmicKind, AlgorithmicSignal};
use super::behavioral::{BehavioralKind, BehavioralSignal, BehavioralSignalSource};
use super::tiered::{ConfidenceTier, build_tiered};
use super::{BoundaryCategory, BoundarySignal};
use crate::review::diff::{ChangeStatus, ChangedFile, ChangedRange};
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
    let tiered = build_tiered(&[], &signals, &[], &[]);
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
    let tiered = build_tiered(&[], &[coarse], &[], &[]);
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
    let tiered = build_tiered(&[], &[ast], &[], &[]);
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
    );
    assert_eq!(tiered.maybe.len(), 1);
    assert!(tiered.definitely.is_empty());
}

#[test]
fn noise_tier_fires_only_for_a_large_diff_with_nothing_flagged() {
    let tiered = build_tiered(&[], &[], &[], &large_diff(6));
    assert_eq!(tiered.noise.len(), 1);
    assert_eq!(tiered.noise[0].tier, ConfidenceTier::LargeDiffOrNoise);
}

#[test]
fn noise_tier_is_suppressed_when_a_signal_is_present() {
    let tiered = build_tiered(
        &[],
        &[behavioral(BehavioralKind::FsWriteAdded, "src/f0.rs")],
        &[],
        &large_diff(6),
    );
    assert!(tiered.noise.is_empty());
    assert_eq!(tiered.maybe.len(), 1);
}

#[test]
fn small_diff_with_nothing_flagged_is_silent() {
    let tiered = build_tiered(&[], &[], &[], &large_diff(2));
    assert!(tiered.is_empty());
}
