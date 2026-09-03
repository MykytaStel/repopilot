use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::analysis::{ArchitectureClassifier, ArchitectureContext};
use crate::findings::types::Finding;
use crate::graph::ImportResolutionStats;
use crate::graph::v2::{GraphEdge, GraphNodeId, GraphSnapshot};
use crate::scan::facts::{FileFacts, ScanFacts};

use super::layers::LayerIndex;
use super::packages::PackageIndex;
use super::{NodeInfo, dead_module_finding, relative_path, test_leak_finding};

pub(super) fn build_file_indexes<'a>(
    facts: &'a ScanFacts,
    root: &Path,
    classifier: &ArchitectureClassifier,
) -> (
    HashMap<PathBuf, ArchitectureContext>,
    HashMap<PathBuf, &'a FileFacts>,
    HashSet<PathBuf>,
) {
    let mut file_context = HashMap::new();
    let mut facts_by_path = HashMap::new();
    let mut known_files = HashSet::new();
    for file in &facts.files {
        let context = classifier.classify(file);
        let abs_path = root.join(&file.path);
        file_context.insert(abs_path.clone(), context.clone());
        file_context.insert(file.path.clone(), context);
        facts_by_path.insert(abs_path, file);
        facts_by_path.insert(file.path.clone(), file);
        known_files.insert(crate::graph::resolver::normalize_path(&file.path));
    }
    (file_context, facts_by_path, known_files)
}

pub(super) fn build_node_info<'a>(
    snapshot: &GraphSnapshot,
    path_by_id: &std::collections::BTreeMap<GraphNodeId, PathBuf>,
    root: &Path,
    file_context: &HashMap<PathBuf, ArchitectureContext>,
    facts_by_path: &HashMap<PathBuf, &'a FileFacts>,
) -> HashMap<GraphNodeId, NodeInfo<'a>> {
    snapshot
        .nodes
        .iter()
        .filter_map(|node| {
            let path = path_by_id.get(&node.id)?;
            let context = file_context.get(path)?;
            let file_facts = facts_by_path.get(path)?;
            Some((
                node.id.clone(),
                NodeInfo {
                    relative: relative_path(path, root),
                    context: context.clone(),
                    facts: Some(*file_facts),
                },
            ))
        })
        .collect()
}

pub(super) fn fan_in_by_target(edges: &[GraphEdge]) -> HashMap<GraphNodeId, usize> {
    let mut fan_in = HashMap::new();
    for edge in edges {
        *fan_in.entry(edge.to.clone()).or_insert(0) += 1;
    }
    fan_in
}

pub(super) fn dead_module_findings(
    snapshot: &GraphSnapshot,
    node_info: &HashMap<GraphNodeId, NodeInfo<'_>>,
    fan_in: &HashMap<GraphNodeId, usize>,
    capabilities: &crate::graph::v2::GraphCapabilities,
    resolution: &ImportResolutionStats,
) -> Vec<Finding> {
    snapshot
        .nodes
        .iter()
        .filter_map(|node| {
            let info = node_info.get(&node.id)?;
            dead_module_finding(
                info,
                fan_in.get(&node.id).copied(),
                super::target_absence_readiness(info, capabilities, resolution),
            )
        })
        .collect()
}

pub(super) fn edge_findings(
    snapshot: &GraphSnapshot,
    node_info: &HashMap<GraphNodeId, NodeInfo<'_>>,
    root: &Path,
    known_files: &HashSet<PathBuf>,
    layer_index: &LayerIndex,
    package_index: &PackageIndex,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut reported_edges = HashSet::new();
    for edge in &snapshot.edges {
        if !reported_edges.insert((edge.from.clone(), edge.to.clone())) {
            continue;
        }
        let (Some(source), Some(target)) = (node_info.get(&edge.from), node_info.get(&edge.to))
        else {
            continue;
        };
        if let Some(finding) = test_leak_finding(source, target, root, known_files) {
            findings.push(finding);
        }
        if let Some(finding) = layer_index.violation_finding(source, target, root, known_files) {
            findings.push(finding);
        }
        if let Some(finding) = package_index.violation_finding(source, target, root, known_files) {
            findings.push(finding);
        }
    }
    findings
}
