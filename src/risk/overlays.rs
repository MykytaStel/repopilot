use crate::baseline::diff::{BaselineStatus, FindingBaselineStatus};
use crate::baseline::key::normalized_relative_path;
use crate::findings::types::Finding;
use crate::graph::CouplingGraph;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::model::{
    FORMULA_VERSION, GraphImpact, RiskInputs, RiskSignal, clamp_score, priority_for_score, signal,
};

mod support;

pub fn apply_baseline_overlay(
    findings: &mut [Finding],
    statuses: &[FindingBaselineStatus],
    root: &Path,
) {
    let status_by_key = statuses
        .iter()
        .map(|status| (status.key.as_str(), status.status))
        .collect::<HashMap<_, _>>();

    for finding in findings {
        let key = crate::baseline::key::stable_finding_key(finding, root);
        if let Some(status) = status_by_key.get(key.as_str()).copied() {
            apply_overlay_signal(
                finding,
                baseline_signal(status),
                RiskInputs {
                    baseline_status: Some(status),
                    ..RiskInputs::default()
                },
            );
        }
    }
}

pub fn apply_review_overlay(findings: &mut [Finding], in_diff: &[bool]) {
    for (finding, in_diff) in findings.iter_mut().zip(in_diff.iter().copied()) {
        if in_diff {
            apply_overlay_signal(
                finding,
                signal(
                    "review.in-diff",
                    "review diff",
                    12,
                    "finding touches changed diff lines",
                ),
                RiskInputs {
                    in_diff: true,
                    ..RiskInputs::default()
                },
            );
        }
    }
}

pub fn apply_workspace_hotspot_overlay(findings: &mut [Finding]) {
    let hotspot_packages = support::workspace_hotspots(findings);
    if hotspot_packages.is_empty() {
        return;
    }

    for finding in findings {
        if finding
            .workspace_package
            .as_ref()
            .is_some_and(|package| hotspot_packages.contains(package))
        {
            apply_overlay_signal(
                finding,
                signal(
                    "workspace.hotspot",
                    "workspace",
                    5,
                    "workspace package has multiple high-risk findings",
                ),
                RiskInputs {
                    workspace_hotspot: true,
                    ..RiskInputs::default()
                },
            );
        }
    }
}

pub fn apply_graph_overlay(findings: &mut [Finding], graph: &CouplingGraph) {
    let impacts = support::graph_impacts(graph);
    if impacts.is_empty() {
        return;
    }

    for finding in findings {
        let Some(path) = support::finding_path_key(finding) else {
            continue;
        };
        let Some(impact) = impacts.get(path.as_str()).copied() else {
            continue;
        };
        let signal = match impact {
            GraphImpact::Hub => signal("graph.hub", "graph", 8, "file is an import hub"),
            GraphImpact::Dependency => signal(
                "graph.dependency",
                "graph",
                5,
                "file is a shared dependency",
            ),
        };
        apply_overlay_signal(
            finding,
            signal,
            RiskInputs {
                graph_impact: Some(impact),
                ..RiskInputs::default()
            },
        );
    }
}

pub fn apply_blast_radius_overlay(
    findings: &mut [Finding],
    repo_root: &Path,
    blast_radius: &[PathBuf],
) {
    let impacted = blast_radius
        .iter()
        .map(|path| support::path_key(path))
        .collect::<HashSet<_>>();
    if impacted.is_empty() {
        return;
    }

    for finding in findings {
        if finding
            .evidence
            .first()
            .map(|evidence| normalized_relative_path(&evidence.path, repo_root))
            .is_some_and(|path| impacted.contains(&path))
        {
            apply_overlay_signal(
                finding,
                signal(
                    "review.blast-radius",
                    "review diff",
                    6,
                    "finding is in a file impacted by changed import dependencies",
                ),
                RiskInputs {
                    blast_radius: true,
                    ..RiskInputs::default()
                },
            );
        }
    }
}

pub fn apply_cluster_overlay(findings: &mut [Finding]) {
    let cluster_sizes = support::repeated_cluster_sizes(findings);
    if cluster_sizes.is_empty() {
        return;
    }

    for finding in findings {
        let Some(key) = support::finding_cluster_key(finding) else {
            continue;
        };
        let Some(size) = cluster_sizes.get(&key).copied().filter(|size| *size >= 3) else {
            continue;
        };
        apply_overlay_signal(
            finding,
            signal(
                "cluster.repeated",
                "cluster",
                support::cluster_weight(size),
                "same rule appears repeatedly in the same repository area",
            ),
            RiskInputs {
                cluster_size: size,
                ..RiskInputs::default()
            },
        );
    }
}

pub(super) fn baseline_signal(status: BaselineStatus) -> RiskSignal {
    match status {
        BaselineStatus::New => signal(
            "baseline.new",
            "baseline",
            10,
            "new findings should be prioritized over accepted existing debt",
        ),
        BaselineStatus::Existing => signal(
            "baseline.existing",
            "baseline",
            -8,
            "existing baseline findings are already accepted technical debt",
        ),
    }
}

fn apply_overlay_signal(finding: &mut Finding, signal: RiskSignal, _inputs: RiskInputs) {
    if finding
        .risk
        .signals
        .iter()
        .any(|existing| existing.id == signal.id)
    {
        return;
    }

    let score = finding.risk.score as i16 + signal.weight;
    finding.risk.signals.push(signal);
    finding.risk.score = clamp_score(score);
    finding.risk.priority = priority_for_score(finding.risk.score);
    finding.risk.formula_version = FORMULA_VERSION.to_string();
}

// Derivation helpers (workspace hotspots, graph impacts, cluster sizing, path
// keys) live in the `support` submodule.
