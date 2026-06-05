//! Derivation helpers for the risk overlays in the parent `overlays` module:
//! workspace hotspots, graph impacts, repeated-cluster sizing, and the path/key
//! normalization they share.

use crate::findings::types::{Finding, Severity};
use crate::graph::{CouplingGraph, compute_metrics};
use crate::risk::model::GraphImpact;
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path};

pub(super) fn workspace_hotspots(findings: &[Finding]) -> HashSet<String> {
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

pub(super) fn graph_impacts(graph: &CouplingGraph) -> HashMap<String, GraphImpact> {
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

pub(super) fn repeated_cluster_sizes(findings: &[Finding]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for finding in findings {
        if let Some(key) = finding_cluster_key(finding) {
            *counts.entry(key).or_default() += 1;
        }
    }
    counts.retain(|_, count| *count >= 3);
    counts
}

pub(super) fn finding_cluster_key(finding: &Finding) -> Option<String> {
    let path = finding
        .evidence
        .first()
        .map(|evidence| path_key(&evidence.path))?;
    Some(format!("{}:{}", finding.rule_id, cluster_scope(&path)))
}

pub(super) fn finding_path_key(finding: &Finding) -> Option<String> {
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

pub(super) fn path_key(path: &Path) -> String {
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

pub(super) fn cluster_weight(size: usize) -> i16 {
    match size {
        0..=2 => 0,
        3..=5 => 3,
        6..=15 => 5,
        _ => 7,
    }
}
