use crate::baseline::diff::{BaselineStatus, FindingBaselineStatus};
use crate::baseline::key::normalized_relative_path;
use crate::findings::types::{Finding, Severity};
use crate::graph::{CouplingGraph, compute_metrics};
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

use super::model::{
    FORMULA_VERSION, GraphImpact, RiskInputs, RiskSignal, clamp_score, priority_for_score, signal,
};

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
                    "changed lines",
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
    let hotspot_packages = workspace_hotspots(findings);
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
                    "workspace hotspot",
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
    let impacts = graph_impacts(graph);
    if impacts.is_empty() {
        return;
    }

    for finding in findings {
        let Some(path) = finding_path_key(finding) else {
            continue;
        };
        let Some(impact) = impacts.get(path.as_str()).copied() else {
            continue;
        };
        let signal = match impact {
            GraphImpact::Hub => signal(
                "graph.hub",
                "dependency hub",
                8,
                "file has high fan-in or fan-out, so changes can ripple through the codebase",
            ),
            GraphImpact::Dependency => signal(
                "graph.dependency",
                "shared dependency",
                5,
                "file is imported by multiple other files",
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
        .map(|path| path_key(path))
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
                    "blast radius",
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
    let cluster_sizes = repeated_cluster_sizes(findings);
    if cluster_sizes.is_empty() {
        return;
    }

    for finding in findings {
        let Some(key) = finding_cluster_key(finding) else {
            continue;
        };
        let Some(size) = cluster_sizes.get(&key).copied().filter(|size| *size >= 3) else {
            continue;
        };
        apply_overlay_signal(
            finding,
            signal(
                "cluster.repeated",
                "repeated pattern",
                cluster_weight(size),
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
            "new finding",
            10,
            "new findings should be prioritized over accepted existing debt",
        ),
        BaselineStatus::Existing => signal(
            "baseline.existing",
            "existing finding",
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

fn workspace_hotspots(findings: &[Finding]) -> HashSet<String> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for finding in findings {
        if finding.severity >= Severity::High
            && let Some(package) = &finding.workspace_package
        {
            *counts.entry(package.clone()).or_default() += 1;
        }
    }
    counts
        .into_iter()
        .filter_map(|(package, count)| (count > 1).then_some(package))
        .collect()
}

fn graph_impacts(graph: &CouplingGraph) -> HashMap<String, GraphImpact> {
    compute_metrics(graph)
        .into_iter()
        .filter_map(|metric| {
            let impact = if metric.fan_in >= 3 || metric.fan_out >= 8 {
                GraphImpact::Hub
            } else if metric.fan_in >= 2 {
                GraphImpact::Dependency
            } else {
                return None;
            };
            Some((path_key(&metric.path), impact))
        })
        .collect()
}

fn repeated_cluster_sizes(findings: &[Finding]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for finding in findings {
        if let Some(key) = finding_cluster_key(finding) {
            *counts.entry(key).or_default() += 1;
        }
    }
    counts.retain(|_, count| *count >= 3);
    counts
}

fn finding_cluster_key(finding: &Finding) -> Option<String> {
    let path = finding
        .evidence
        .first()
        .map(|evidence| path_key(&evidence.path))?;
    Some(format!("{}:{}", finding.rule_id, cluster_scope(&path)))
}

fn finding_path_key(finding: &Finding) -> Option<String> {
    finding
        .evidence
        .first()
        .map(|evidence| path_key(&evidence.path))
}

fn cluster_scope(path: &str) -> String {
    let mut parts = path.split('/').filter(|part| !part.is_empty());
    match (parts.next(), parts.next()) {
        (Some(first), Some(second)) if second.contains('.') => first.to_string(),
        (Some(first), Some(second)) => format!("{first}/{second}"),
        (Some(first), None) => first.to_string(),
        _ => ".".to_string(),
    }
}

fn path_key(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::CurDir => None,
            Component::Normal(value) => Some(value.to_string_lossy().to_string()),
            Component::RootDir | Component::Prefix(_) | Component::ParentDir => {
                Some(component.as_os_str().to_string_lossy().to_string())
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn cluster_weight(size: usize) -> i16 {
    match size {
        0..=2 => 0,
        3..=5 => 3,
        6..=15 => 5,
        _ => 7,
    }
}
