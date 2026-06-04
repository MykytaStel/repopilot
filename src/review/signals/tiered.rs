//! Unified, confidence-tiered view over the review's change signals.
//!
//! Boundary, behavioral, and algorithmic signals all converge into one
//! [`ReviewSignal`] type, grouped into three trust tiers so a reviewer's eye
//! goes to the right place first:
//!
//! - **definitely sensitive** — boundary changes and unambiguous behavior
//!   crossings (env var, migration, subprocess, removed error handling / auth /
//!   test, a network call in an auth path).
//! - **maybe sensitive** — behavior in ordinary files (network call, fs write,
//!   new dependency, raw SQL) and every algorithmic delta.
//! - **large-diff-or-noise** — a big diff with nothing flagged, surfaced once so
//!   volume is visible without pretending to understand it.
//!
//! Humble by construction: every signal carries a structural fact, never a
//! verdict. The existing `boundary_signals` view is kept untouched; this is an
//! additive, forward-looking surface.

use crate::review::diff::ChangedFile;
use crate::review::paths::normalized_review_path;
use crate::review::signals::BoundarySignal;
use crate::review::signals::algorithmic::{AlgorithmicKind, AlgorithmicSignal};
use crate::review::signals::behavioral::{BehavioralKind, BehavioralSignal};
use crate::review::signals::composites;
use crate::scan::types::CouplingGraph;
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::Path;

/// A big diff is only called out as "noise" once it crosses both of these and
/// nothing else was flagged.
const LARGE_DIFF_FILES: usize = 5;
const LARGE_DIFF_LINES: usize = 200;

/// How much a reviewer should trust that a signal points at something worth a look.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConfidenceTier {
    DefinitelySensitive,
    MaybeSensitive,
    LargeDiffOrNoise,
}

/// Which detector produced a signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SignalFamily {
    Boundary,
    Behavioral,
    Algorithmic,
    /// The single large-diff/volume note, which belongs to no detector family.
    Volume,
}

/// One unified review signal, ready to render under its tier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReviewSignal {
    pub family: SignalFamily,
    pub tier: ConfidenceTier,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    /// Neutral structural fact, e.g. "network call added" / "nesting 2 → 4".
    pub headline: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// How many files import the signal's file (reach), from the coupling graph.
    pub blast_radius: usize,
}

/// The review's signals grouped by confidence tier.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct TieredSignals {
    pub definitely: Vec<ReviewSignal>,
    pub maybe: Vec<ReviewSignal>,
    pub noise: Vec<ReviewSignal>,
}

impl TieredSignals {
    fn push(&mut self, signal: ReviewSignal) {
        match signal.tier {
            ConfidenceTier::DefinitelySensitive => self.definitely.push(signal),
            ConfidenceTier::MaybeSensitive => self.maybe.push(signal),
            ConfidenceTier::LargeDiffOrNoise => self.noise.push(signal),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.definitely.is_empty() && self.maybe.is_empty() && self.noise.is_empty()
    }

    pub fn total(&self) -> usize {
        self.definitely.len() + self.maybe.len() + self.noise.len()
    }

    fn sort(&mut self) {
        for group in [&mut self.definitely, &mut self.maybe, &mut self.noise] {
            group.sort_by(|left, right| {
                left.path
                    .cmp(&right.path)
                    .then(left.line.cmp(&right.line))
                    .then(left.headline.cmp(&right.headline))
            });
        }
    }
}

/// Fold the three signal families into one tiered view.
pub fn build_tiered(
    boundary: &[BoundarySignal],
    behavioral: &[BehavioralSignal],
    algorithmic: &[AlgorithmicSignal],
    changed_files: &[ChangedFile],
) -> TieredSignals {
    let mut tiered = TieredSignals::default();

    // Files that decide who-can-do-what; a network call added *there* is more
    // than maybe-sensitive.
    let access_paths: BTreeSet<&str> = boundary
        .iter()
        .filter(|signal| signal.category.is_code_boundary())
        .map(|signal| signal.path.as_str())
        .collect();

    for signal in boundary {
        tiered.push(from_boundary(signal));
    }
    for signal in behavioral {
        let in_access = access_paths.contains(signal.path.as_str());
        tiered.push(ReviewSignal {
            family: SignalFamily::Behavioral,
            tier: behavioral_tier(signal.kind, in_access, signal.is_coarse()),
            path: signal.path.clone(),
            line: Some(signal.line),
            headline: behavioral_headline(signal.kind).to_string(),
            detail: Some(signal.detail.clone()),
            blast_radius: 0,
        });
    }
    for signal in algorithmic {
        tiered.push(ReviewSignal {
            family: SignalFamily::Algorithmic,
            tier: ConfidenceTier::MaybeSensitive,
            path: signal.path.clone(),
            line: Some(signal.line),
            headline: algorithmic_headline(signal.kind).to_string(),
            detail: Some(signal.detail.clone()),
            blast_radius: 0,
        });
    }

    // Only call out raw volume when nothing else fired — otherwise the flagged
    // signals already point the eye.
    if tiered.definitely.is_empty() && tiered.maybe.is_empty() {
        let (files, lines) = diff_size(changed_files);
        if files > LARGE_DIFF_FILES && lines > LARGE_DIFF_LINES {
            tiered.push(ReviewSignal {
                family: SignalFamily::Volume,
                tier: ConfidenceTier::LargeDiffOrNoise,
                path: String::new(),
                line: None,
                headline: "large diff, no boundary or behavior flagged".to_string(),
                detail: Some(format!("{files} files, ~{lines} changed lines")),
                blast_radius: 0,
            });
        }
    }

    tiered.sort();
    tiered
}

/// Fill each signal's reach (importer count) from the coupling graph, mirroring
/// the boundary-signal enrichment. No-op without a graph; boundary signals keep
/// the reach they were already given.
pub fn enrich_blast_radius(
    tiered: &mut TieredSignals,
    graph: Option<&CouplingGraph>,
    repo_root: &Path,
) {
    let Some(graph) = graph else {
        return;
    };
    let importers = composites::build_importers_by_target(graph, repo_root);

    for group in [&mut tiered.definitely, &mut tiered.maybe, &mut tiered.noise] {
        for signal in group.iter_mut() {
            if signal.blast_radius != 0 || signal.path.is_empty() {
                continue;
            }
            let normalized = normalized_review_path(Path::new(&signal.path), repo_root);
            signal.blast_radius = importers.get(&normalized).map_or(0, BTreeSet::len);
        }
    }
}

fn from_boundary(signal: &BoundarySignal) -> ReviewSignal {
    ReviewSignal {
        family: SignalFamily::Boundary,
        tier: ConfidenceTier::DefinitelySensitive,
        path: signal.path.clone(),
        line: None,
        headline: format!("{} changed", signal.category.label()),
        detail: None,
        blast_radius: signal.blast_radius,
    }
}

fn behavioral_tier(kind: BehavioralKind, in_access_boundary: bool, coarse: bool) -> ConfidenceTier {
    use BehavioralKind::*;
    // Coarse (non-AST) signals are best-effort hints from a file we couldn't
    // parse; keep them visible but never above the large-diff/noise tier.
    if coarse {
        return ConfidenceTier::LargeDiffOrNoise;
    }
    match kind {
        SubprocessAdded | EnvVarIntroduced | MigrationAdded | ErrorHandlingRemoved
        | TestDeletedOrEmptied | AuthCheckRemoved => ConfidenceTier::DefinitelySensitive,
        NetworkCallAdded if in_access_boundary => ConfidenceTier::DefinitelySensitive,
        NetworkCallAdded | FsWriteAdded | DependencyImportAdded | RawSqlAdded => {
            ConfidenceTier::MaybeSensitive
        }
    }
}

fn behavioral_headline(kind: BehavioralKind) -> &'static str {
    use BehavioralKind::*;
    match kind {
        NetworkCallAdded => "network call added",
        SubprocessAdded => "subprocess/exec added",
        FsWriteAdded => "filesystem write added",
        EnvVarIntroduced => "env var introduced",
        DependencyImportAdded => "dependency import added",
        MigrationAdded => "migration added",
        RawSqlAdded => "raw SQL added",
        ErrorHandlingRemoved => "error handling removed",
        TestDeletedOrEmptied => "test deleted or emptied",
        AuthCheckRemoved => "auth check removed",
    }
}

fn algorithmic_headline(kind: AlgorithmicKind) -> &'static str {
    use AlgorithmicKind::*;
    match kind {
        ComplexityIncreased => "complexity increased",
        NestedLoopIntroduced => "nested loop introduced",
        FunctionGrew => "function grew",
        RecursionIntroduced => "recursion introduced",
    }
}

fn diff_size(changed_files: &[ChangedFile]) -> (usize, usize) {
    let lines = changed_files
        .iter()
        .flat_map(|file| file.ranges.iter())
        .map(|range| range.end.saturating_sub(range.start) + 1)
        .sum();
    (changed_files.len(), lines)
}
