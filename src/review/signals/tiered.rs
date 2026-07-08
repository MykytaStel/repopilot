//! Unified, confidence-tiered view over the review's change signals.
//!
//! Boundary, behavioral, algorithmic, and taint-lite signals converge into one
//! [`ReviewSignal`] type, grouped into three trust tiers so a reviewer's eye
//! goes to the right place first:
//!
//! - **definitely sensitive** — boundary changes and unambiguous behavior
//!   crossings (env var, migration, subprocess, removed error handling / auth /
//!   test, a network call in an auth path), plus input reaching raw SQL or exec.
//! - **maybe sensitive** — behavior in ordinary files (network call, fs write,
//!   new dependency, raw SQL), every algorithmic delta, and input reaching a
//!   filesystem write or outbound network call.
//! - **large-diff-or-noise** — a big diff with nothing flagged, surfaced once so
//!   volume is visible without pretending to understand it.
//!
//! Humble by construction: every signal carries a structural fact, never a
//! verdict. The existing `boundary_signals` view is kept untouched; this is an
//! additive, forward-looking surface.

use crate::findings::provenance::AnalysisScope;
use crate::findings::types::Confidence;
use crate::review::diff::ChangedFile;
use crate::review::paths::normalized_review_path;
use crate::review::signals::BoundarySignal;
use crate::review::signals::algorithmic::{AlgorithmicKind, AlgorithmicSignal};
use crate::review::signals::behavioral::{BehavioralKind, BehavioralSignal};
use crate::review::signals::composites;
use crate::review::signals::taint::{SinkKind, TaintSignal};
use crate::rules::{RuleLifecycle, SignalSource};
use crate::scan::cache::stable_hash_hex;
use crate::scan::types::CouplingGraph;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
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
    /// Untrusted input reaching a dangerous sink (taint-lite reachability).
    Taint,
    /// The single large-diff/volume note, which belongs to no detector family.
    Volume,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReviewSignalProvenance {
    pub detector: String,
    pub lifecycle: RuleLifecycle,
    pub signal_source: SignalSource,
    pub analysis_scope: AnalysisScope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReviewSignalVerificationPlan {
    pub steps: Vec<String>,
}

/// One unified review signal, ready to render under its tier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReviewSignal {
    pub signal_id: String,
    pub kind: String,
    pub family: SignalFamily,
    pub tier: ConfidenceTier,
    pub confidence: Confidence,
    pub path: String,
    /// Compatibility alias retained throughout the pre-1.0 schema line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_start: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_end: Option<usize>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub evidence_lines: Vec<usize>,
    /// Neutral structural fact, e.g. "network call added" / "nesting 2 → 4".
    pub headline: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// How many files import the signal's file (reach), from the coupling graph.
    pub blast_radius: usize,
    pub provenance: ReviewSignalProvenance,
    pub suppressed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suppression_reason: Option<String>,
    pub gate_eligible: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_plan: Option<ReviewSignalVerificationPlan>,
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

    pub fn gate_eligible_definitely_count(&self) -> usize {
        self.definitely
            .iter()
            .filter(|signal| signal.gate_eligible && !signal.suppressed)
            .count()
    }

    /// Whether any visible tier carries a taint-lite reachability signal. Used to
    /// surface the "a path exists, not a confirmed vulnerability" disclaimer once
    /// per report rather than repeating it on every taint row.
    pub fn has_taint_signal(&self) -> bool {
        [&self.definitely, &self.maybe, &self.noise]
            .into_iter()
            .flatten()
            .any(|signal| !signal.suppressed && signal.family == SignalFamily::Taint)
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut ReviewSignal> {
        self.definitely
            .iter_mut()
            .chain(self.maybe.iter_mut())
            .chain(self.noise.iter_mut())
    }
}

/// Fold the four signal families into one tiered view.
pub fn build_tiered(
    boundary: &[BoundarySignal],
    behavioral: &[BehavioralSignal],
    algorithmic: &[AlgorithmicSignal],
    taint: &[TaintSignal],
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
        tiered.push(build_signal(
            behavioral_kind(signal.kind),
            SignalFamily::Behavioral,
            behavioral_tier(signal.kind, in_access, signal.is_coarse()),
            if signal.is_coarse() {
                Confidence::Low
            } else {
                Confidence::High
            },
            signal.path.clone(),
            Some(signal.line),
            behavioral_headline(signal.kind),
            Some(signal.detail.clone()),
            if signal.is_coarse() {
                SignalSource::TextHeuristic
            } else {
                SignalSource::Ast
            },
        ));
    }
    for signal in algorithmic {
        tiered.push(build_signal(
            algorithmic_kind(signal.kind),
            SignalFamily::Algorithmic,
            ConfidenceTier::MaybeSensitive,
            Confidence::Medium,
            signal.path.clone(),
            Some(signal.line),
            algorithmic_headline(signal.kind),
            Some(signal.detail.clone()),
            SignalSource::Ast,
        ));
    }
    for signal in taint {
        tiered.push(build_signal(
            taint_kind(signal.sink),
            SignalFamily::Taint,
            taint_tier(signal.sink),
            Confidence::High,
            signal.path.clone(),
            Some(signal.line),
            taint_headline(signal.sink),
            Some(signal.detail.clone()),
            SignalSource::Ast,
        ));
    }

    // Only call out raw volume when nothing else fired — otherwise the flagged
    // signals already point the eye.
    if tiered.definitely.is_empty() && tiered.maybe.is_empty() {
        let (files, lines) = diff_size(changed_files);
        if files > LARGE_DIFF_FILES && lines > LARGE_DIFF_LINES {
            tiered.push(build_signal(
                "volume.large-diff",
                SignalFamily::Volume,
                ConfidenceTier::LargeDiffOrNoise,
                Confidence::Low,
                String::new(),
                None,
                "large diff, no review signal flagged",
                Some(format!("{files} files, ~{lines} changed lines")),
                SignalSource::GitDiff,
            ));
        }
    }

    deduplicate(&mut tiered);
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
    let mut review_signal = build_signal(
        boundary_kind(signal.category),
        SignalFamily::Boundary,
        ConfidenceTier::DefinitelySensitive,
        Confidence::High,
        signal.path.clone(),
        None,
        &format!("{} changed", signal.category.label()),
        None,
        SignalSource::GitDiff,
    );
    review_signal.blast_radius = signal.blast_radius;
    review_signal
}

#[allow(clippy::too_many_arguments)]
fn build_signal(
    kind: &str,
    family: SignalFamily,
    tier: ConfidenceTier,
    confidence: Confidence,
    path: String,
    line: Option<usize>,
    headline: &str,
    detail: Option<String>,
    signal_source: SignalSource,
) -> ReviewSignal {
    let signal_id = stable_hash_hex(format!("{kind}\0{path}").as_bytes())[..16].to_string();
    let gate_eligible = tier == ConfidenceTier::DefinitelySensitive;
    let verification_plan = build_verification_plan(
        kind,
        family,
        confidence,
        &path,
        line,
        headline,
        detail.as_deref(),
        gate_eligible,
    );

    ReviewSignal {
        signal_id,
        kind: kind.to_string(),
        family,
        tier,
        confidence,
        path,
        line,
        line_start: line,
        line_end: line,
        evidence_lines: line.into_iter().collect(),
        headline: headline.to_string(),
        detail,
        blast_radius: 0,
        provenance: ReviewSignalProvenance {
            detector: kind.to_string(),
            lifecycle: RuleLifecycle::Preview,
            signal_source,
            analysis_scope: AnalysisScope::GitDiff,
        },
        suppressed: false,
        suppression_reason: None,
        gate_eligible,
        verification_plan,
    }
}

#[allow(clippy::too_many_arguments)]
fn build_verification_plan(
    kind: &str,
    family: SignalFamily,
    confidence: Confidence,
    path: &str,
    line: Option<usize>,
    headline: &str,
    detail: Option<&str>,
    gate_eligible: bool,
) -> Option<ReviewSignalVerificationPlan> {
    if !gate_eligible || confidence != Confidence::High || path.is_empty() {
        return None;
    }

    let location = match line {
        Some(line) => format!("{path}:{line}"),
        None => path.to_string(),
    };
    let mut steps = vec![format!(
        "Open {location} and confirm the changed diff still supports the review signal: {headline}."
    )];

    if let Some(detail) = detail.filter(|detail| !detail.trim().is_empty()) {
        steps.push(format!(
            "Compare the pre-change and post-change code around {location}; verify this detail is still accurate: {detail}."
        ));
    }

    steps.push(family_specific_verification_step(kind, family).to_string());
    steps.push(
        "This plan is generated from Git diff/static review evidence only — it does not execute code, run tests, or observe runtime behavior. Treat it as a starting point for manual or test-based confirmation, not proof."
            .to_string(),
    );

    Some(ReviewSignalVerificationPlan { steps })
}

fn family_specific_verification_step(kind: &str, family: SignalFamily) -> &'static str {
    match family {
        SignalFamily::Boundary => match kind {
            "boundary.access-control" => {
                "Confirm the changed access-control path still enforces the expected authentication, authorization, session, or permission behavior, and add or update a focused test when coverage is missing."
            }
            "boundary.request-trust" => {
                "Confirm the changed request-trust surface still enforces the expected CORS, CSP, security-header, cookie, or request validation behavior."
            }
            "boundary.deploy-surface" => {
                "Review the changed deploy or CI surface for privilege, secret, environment, artifact, and production rollout impact before merging."
            }
            "boundary.supply-chain" => {
                "Review the dependency or lockfile change, confirm the package source and version are expected, and run the project's dependency/security checks."
            }
            "boundary.secret-config" => {
                "Confirm no real secret was committed and that runtime configuration still comes from the intended secret-management path."
            }
            _ => {
                "Review the changed boundary with the project owner and confirm it is covered by tests, configuration review, or a manual release checklist."
            }
        },
        SignalFamily::Behavioral => match kind {
            "behavioral.network-call-added" => {
                "Confirm the new network call has the expected timeout, retry, error handling, authorization, and data exposure behavior."
            }
            "behavioral.subprocess-added" => {
                "Confirm subprocess arguments cannot be controlled unsafely by user input and that failures are handled explicitly."
            }
            "behavioral.fs-write-added" => {
                "Confirm the filesystem write target is expected, path traversal is not possible, and failures do not leave partial unsafe state."
            }
            "behavioral.env-var-introduced" => {
                "Confirm the new environment variable is documented, validated at startup, and safe when missing or malformed."
            }
            "behavioral.migration-added" => {
                "Review the migration forward/backward path, data-loss risk, locking behavior, and deployment ordering."
            }
            "behavioral.error-handling-removed" => {
                "Confirm removing this error handling cannot hide failures, skip cleanup, or turn recoverable errors into unsafe success paths."
            }
            "behavioral.auth-check-removed" => {
                "Confirm the removed auth check is replaced by an equivalent guard in the same request path or an earlier boundary."
            }
            _ => {
                "Confirm the behavioral change is intentional and covered by a focused test or a documented manual check."
            }
        },
        SignalFamily::Taint => match kind {
            "taint.sql" => {
                "Trace the changed input source to the SQL sink and confirm parameterization, escaping, validation, or an equivalent guard blocks injection."
            }
            "taint.exec" => {
                "Trace the changed input source to the subprocess sink and confirm arguments are fixed, allowlisted, or safely encoded."
            }
            _ => {
                "Trace the changed input source to the reported sink and confirm validation, allowlisting, or safe encoding exists on that path."
            }
        },
        SignalFamily::Algorithmic => {
            "Confirm the algorithmic change is intentional, covered by representative inputs, and does not regress the expected complexity envelope."
        }
        SignalFamily::Volume => {
            "Split or manually sample the large diff so ownership, tests, and risky surfaces are reviewed instead of treating volume as proof."
        }
    }
}

fn deduplicate(tiered: &mut TieredSignals) {
    let mut groups: BTreeMap<(String, String), ReviewSignal> = BTreeMap::new();
    for signal in tiered
        .definitely
        .drain(..)
        .chain(tiered.maybe.drain(..))
        .chain(tiered.noise.drain(..))
    {
        let key = (signal.kind.clone(), signal.path.clone());
        groups
            .entry(key)
            .and_modify(|existing| {
                existing.evidence_lines.extend(signal.evidence_lines.iter());
                existing.evidence_lines.sort_unstable();
                existing.evidence_lines.dedup();
                existing.line_start = existing.evidence_lines.first().copied();
                existing.line_end = existing.evidence_lines.last().copied();
                existing.line = existing.line_start;
                existing.blast_radius = existing.blast_radius.max(signal.blast_radius);
                if existing.verification_plan.is_none() {
                    existing.verification_plan = signal.verification_plan.clone();
                }
            })
            .or_insert(signal);
    }
    for signal in groups.into_values() {
        tiered.push(signal);
    }
}

fn boundary_kind(category: crate::review::signals::BoundaryCategory) -> &'static str {
    use crate::review::signals::BoundaryCategory::*;
    match category {
        AccessControl => "boundary.access-control",
        RequestTrust => "boundary.request-trust",
        DeploySurface => "boundary.deploy-surface",
        SupplyChain => "boundary.supply-chain",
        SecretConfig => "boundary.secret-config",
        Custom => "boundary.custom",
    }
}

fn behavioral_kind(kind: BehavioralKind) -> &'static str {
    use BehavioralKind::*;
    match kind {
        NetworkCallAdded => "behavioral.network-call-added",
        SubprocessAdded => "behavioral.subprocess-added",
        FsWriteAdded => "behavioral.fs-write-added",
        EnvVarIntroduced => "behavioral.env-var-introduced",
        DependencyImportAdded => "behavioral.dependency-import-added",
        MigrationAdded => "behavioral.migration-added",
        RawSqlAdded => "behavioral.raw-sql-added",
        ErrorHandlingRemoved => "behavioral.error-handling-removed",
        TestDeletedOrEmptied => "behavioral.test-deleted-or-emptied",
        AuthCheckRemoved => "behavioral.auth-check-removed",
    }
}

fn algorithmic_kind(kind: AlgorithmicKind) -> &'static str {
    use AlgorithmicKind::*;
    match kind {
        ComplexityIncreased => "algorithmic.complexity-increased",
        NestedLoopIntroduced => "algorithmic.nested-loop-introduced",
        FunctionGrew => "algorithmic.function-grew",
        RecursionIntroduced => "algorithmic.recursion-introduced",
    }
}

fn taint_kind(sink: SinkKind) -> &'static str {
    match sink {
        SinkKind::Sql => "taint.sql",
        SinkKind::Exec => "taint.exec",
        SinkKind::FsWrite => "taint.fs-write",
        SinkKind::Network => "taint.network",
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

/// SQL/exec injection lands in the top tier; filesystem/network reach is real but
/// more often legitimate, so it stays *maybe sensitive*.
fn taint_tier(sink: SinkKind) -> ConfidenceTier {
    match sink {
        SinkKind::Sql | SinkKind::Exec => ConfidenceTier::DefinitelySensitive,
        SinkKind::FsWrite | SinkKind::Network => ConfidenceTier::MaybeSensitive,
    }
}

fn taint_headline(sink: SinkKind) -> &'static str {
    match sink {
        SinkKind::Sql => "untrusted input reaches raw SQL",
        SinkKind::Exec => "untrusted input reaches subprocess/exec",
        SinkKind::FsWrite => "untrusted input reaches filesystem write",
        SinkKind::Network => "untrusted input reaches network call",
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
