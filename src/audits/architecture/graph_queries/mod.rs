//! Whole-repo architecture rules that query the import graph: dead modules,
//! test leaks into production, declared-layer violations, and package-boundary
//! violations. The layer and package rules are strictly opt-in — they emit
//! nothing unless the user declares `[[architecture.layers]]` or
//! `[architecture] package_roots`.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::analysis::{ArchitectureClassifier, ArchitectureContext, FileRole};
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::graph::CouplingGraph;
use crate::graph::v2::{GraphNodeId, build_coupling_graph_snapshot};
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;

mod layers;
mod packages;

#[cfg(test)]
mod tests;

use layers::LayerIndex;
use packages::PackageIndex;

pub struct GraphQueriesAudit;

/// A resolved graph node: its repo-relative path and the architecture
/// classification the rules reason over.
pub(crate) struct NodeInfo {
    pub relative: PathBuf,
    pub context: ArchitectureContext,
}

impl GraphQueriesAudit {
    pub fn audit(
        &self,
        facts: &ScanFacts,
        config: &ScanConfig,
        graph: &CouplingGraph,
        root: &Path,
    ) -> Vec<Finding> {
        let classifier = ArchitectureClassifier::new(&config.module_mappings);

        // Classify every file once, keyed by both relative and absolute path so
        // we can match whichever form the snapshot carries.
        let mut file_context: HashMap<PathBuf, ArchitectureContext> = HashMap::new();
        for file in &facts.files {
            let context = classifier.classify(file);
            file_context.insert(root.join(&file.path), context.clone());
            file_context.insert(file.path.clone(), context);
        }

        let (snapshot, path_by_id) = build_coupling_graph_snapshot(graph);

        let mut node_info: HashMap<GraphNodeId, NodeInfo> = HashMap::new();
        for node in &snapshot.nodes {
            if let Some(path) = path_by_id.get(&node.id)
                && let Some(context) = file_context.get(path)
            {
                node_info.insert(
                    node.id.clone(),
                    NodeInfo {
                        relative: relative_path(path, root),
                        context: context.clone(),
                    },
                );
            }
        }

        let mut fan_in: HashMap<GraphNodeId, usize> = HashMap::new();
        for edge in &snapshot.edges {
            *fan_in.entry(edge.to.clone()).or_insert(0) += 1;
        }

        let mut findings = Vec::new();

        for node in &snapshot.nodes {
            if let Some(info) = node_info.get(&node.id)
                && let Some(finding) = dead_module_finding(info, fan_in.get(&node.id).copied())
            {
                findings.push(finding);
            }
        }

        let layer_index = LayerIndex::from_config(config);
        let package_index = PackageIndex::from_config(config);

        let mut reported_edges = HashSet::new();
        for edge in &snapshot.edges {
            if !reported_edges.insert((edge.from.clone(), edge.to.clone())) {
                continue;
            }
            let (Some(source), Some(target)) = (node_info.get(&edge.from), node_info.get(&edge.to))
            else {
                continue;
            };

            if let Some(finding) = test_leak_finding(source, target) {
                findings.push(finding);
            }
            if let Some(finding) = layer_index.violation_finding(source, target) {
                findings.push(finding);
            }
            if let Some(finding) = package_index.violation_finding(source, target) {
                findings.push(finding);
            }
        }

        findings
    }
}

/// A production file that nothing imports and that is not an entrypoint or a
/// package's public API surface is likely dead code.
fn dead_module_finding(info: &NodeInfo, fan_in: Option<usize>) -> Option<Finding> {
    let ctx = &info.context;
    if ctx.file_role != FileRole::Production
        || ctx.is_entrypoint
        || ctx.is_public_api
        || fan_in.unwrap_or(0) != 0
    {
        return None;
    }

    Some(architecture_finding(
        "architecture.dead-module",
        "Dead module detected",
        "This production file is not imported by any other project file and is not a known entrypoint.".to_string(),
        Evidence {
            path: info.relative.clone(),
            line_start: 1,
            line_end: None,
            snippet: "fan_in=0, role=Production, entrypoint=false".to_string(),
        },
    ))
}

/// A production file importing a test or fixture file leaks test-only code into
/// the shipped build.
fn test_leak_finding(source: &NodeInfo, target: &NodeInfo) -> Option<Finding> {
    if source.context.file_role != FileRole::Production {
        return None;
    }
    let kind = match target.context.file_role {
        FileRole::Test => "test",
        FileRole::Fixture => "fixture",
        _ => return None,
    };

    Some(architecture_finding(
        "architecture.test-leak",
        "Test code leaked into production",
        format!("Production file imports a {kind} file."),
        Evidence {
            path: source.relative.clone(),
            line_start: 1,
            line_end: None,
            snippet: format!("Imports: {}", target.relative.display()),
        },
    ))
}

/// Shared constructor for architecture findings. Severity and confidence are
/// left at the `Info`/`Medium` sentinels so the rule registry owns them via
/// `populate_rule_metadata` (single source of truth — no inline severity here).
pub(crate) fn architecture_finding(
    rule_id: &str,
    title: &str,
    description: String,
    evidence: Evidence,
) -> Finding {
    Finding {
        id: String::new(),
        rule_id: rule_id.to_string(),
        recommendation: Finding::recommendation_for_rule_id(rule_id),
        title: title.to_string(),
        description,
        category: FindingCategory::Architecture,
        severity: Severity::Info,
        confidence: Default::default(),
        evidence: vec![evidence],
        workspace_package: None,
        docs_url: None,
        provenance: Default::default(),
        risk: Default::default(),
    }
}

pub(crate) fn relative_path(path: &Path, root: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}
