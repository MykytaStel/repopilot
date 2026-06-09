use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::graph::v2::{
    GraphEdge, GraphEdgeConfidence, GraphEdgeKind, GraphEdgeProvenance, GraphNode, GraphNodeId,
    GraphNodeKind, GraphSnapshot, find_cycles,
};
use crate::graph::{
    CouplingGraph, FileMetrics, build_coupling_graph, compute_metrics,
    without_rust_module_containment_edges,
};
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub struct ImportCouplingAudit;

impl ImportCouplingAudit {
    pub fn audit_with_graph(
        &self,
        facts: &ScanFacts,
        config: &ScanConfig,
        root: &Path,
    ) -> (Vec<Finding>, CouplingGraph) {
        let graph = build_coupling_graph(facts, root);
        let metrics = compute_metrics(&graph);

        // Circular dependencies now run through graph v2's SCC-based cycle
        // detection. We re-encode the existing (module-containment-stripped)
        // coupling graph as a `GraphSnapshot` and reuse `find_cycles`; the v1
        // graph build still feeds fan-out/instability metrics and the graph
        // returned to the scan pipeline. Each cycle is one strongly-connected
        // component (set of mutually dependent files).
        let cycle_graph = without_rust_module_containment_edges(&graph);
        let (cycle_snapshot, path_by_id) = coupling_graph_snapshot(&cycle_graph);
        let cycles: Vec<Vec<PathBuf>> = find_cycles(&cycle_snapshot)
            .into_iter()
            .map(|cycle| {
                cycle
                    .node_ids
                    .iter()
                    .filter_map(|id| path_by_id.get(id).cloned())
                    .collect()
            })
            .collect();

        let classifier = crate::analysis::ArchitectureClassifier::new(&config.module_mappings);
        let mut findings = Vec::new();

        for metric in &metrics {
            let is_prod = facts
                .files
                .iter()
                .find(|file| file.path == metric.path || root.join(&file.path) == metric.path)
                .map(|file| {
                    classifier.classify(file).file_role == crate::analysis::FileRole::Production
                })
                .unwrap_or(true);

            if !is_prod {
                continue;
            }

            if metric.fan_out > config.max_fan_out && !is_pure_rust_facade(metric, facts, root) {
                findings.push(excessive_fan_out_finding(metric, root, config.max_fan_out));
            }

            let instability_pct = (metric.instability * 100.0).round() as usize;
            if metric.fan_in >= config.instability_hub_min_fan_in
                && instability_pct >= config.instability_hub_min_instability_pct
            {
                findings.push(high_instability_hub_finding(
                    metric,
                    root,
                    instability_pct,
                    config.instability_hub_min_fan_in,
                    config.instability_hub_min_instability_pct,
                ));
            }
        }

        let mut seen_cycles = BTreeSet::new();
        for cycle in cycles {
            let is_prod_cycle = cycle.iter().all(|path| {
                facts
                    .files
                    .iter()
                    .find(|file| file.path == *path || root.join(&file.path) == *path)
                    .map(|file| {
                        classifier.classify(file).file_role == crate::analysis::FileRole::Production
                    })
                    .unwrap_or(true)
            });

            if !is_prod_cycle {
                continue;
            }

            if seen_cycles.insert(cycle.clone()) {
                findings.push(circular_dependency_finding(&cycle, root));
            }
        }

        (findings, graph)
    }
}

fn excessive_fan_out_finding(metric: &FileMetrics, root: &Path, threshold: usize) -> Finding {
    let path = relative_path(&metric.path, root);

    Finding {
        id: String::new(),
        rule_id: "architecture.excessive-fan-out".to_string(),
        recommendation: Finding::recommendation_for_rule_id("architecture.excessive-fan-out"),
        title: "File imports too many project files".to_string(),
        description: format!(
            "This file imports {} project files, exceeding the configured fan-out threshold of {threshold}.",
            metric.fan_out
        ),
        category: FindingCategory::Architecture,
        severity: Severity::Medium,
        confidence: Default::default(),
        evidence: vec![Evidence {
            path: path.clone(),
            line_start: 1,
            line_end: None,
            snippet: format!(
                "{} fan_out={}; threshold={threshold}.",
                path.display(),
                metric.fan_out
            ),
        }],
        workspace_package: None,
        docs_url: None,
        provenance: Default::default(),
        risk: Default::default(),
    }
}

fn high_instability_hub_finding(
    metric: &FileMetrics,
    root: &Path,
    instability_pct: usize,
    min_fan_in: usize,
    min_instability_pct: usize,
) -> Finding {
    let path = relative_path(&metric.path, root);

    Finding {
        id: String::new(),
        rule_id: "architecture.high-instability-hub".to_string(),
        recommendation: Finding::recommendation_for_rule_id("architecture.high-instability-hub"),
        title: "Highly unstable import hub".to_string(),
        description: format!(
            "This file is imported by {} files while also importing {} files, making it a highly unstable hub.",
            metric.fan_in, metric.fan_out
        ),
        category: FindingCategory::Architecture,
        severity: Severity::High,
        confidence: Default::default(),
        evidence: vec![Evidence {
            path: path.clone(),
            line_start: 1,
            line_end: None,
            snippet: format!(
                "{} fan_in={}, fan_out={}, instability={}%; thresholds: fan_in>={min_fan_in}, instability>={min_instability_pct}%.",
                path.display(),
                metric.fan_in,
                metric.fan_out,
                instability_pct
            ),
        }],
        workspace_package: None,
        docs_url: None,
        provenance: Default::default(),
        risk: Default::default(),
    }
}

fn circular_dependency_finding(cycle: &[PathBuf], root: &Path) -> Finding {
    let relative_cycle: Vec<PathBuf> = cycle.iter().map(|path| relative_path(path, root)).collect();
    let cycle_path = relative_cycle
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(" -> ");
    let file_count = relative_cycle.len();
    let evidence = relative_cycle
        .iter()
        .map(|path| Evidence {
            path: path.clone(),
            line_start: 1,
            line_end: None,
            snippet: format!("Cycle ({file_count} files): {cycle_path}."),
        })
        .collect();

    Finding {
        id: String::new(),
        rule_id: "architecture.circular-dependency".to_string(),
        recommendation: Finding::recommendation_for_rule_id("architecture.circular-dependency"),
        title: "Circular dependency detected".to_string(),
        description: format!(
            "A circular dependency was detected across {file_count} project files."
        ),
        category: FindingCategory::Architecture,
        severity: Severity::High,
        confidence: Default::default(),
        evidence,
        workspace_package: None,
        docs_url: None,
        provenance: Default::default(),
        risk: Default::default(),
    }
}

fn is_pure_rust_facade(metric: &FileMetrics, facts: &ScanFacts, root: &Path) -> bool {
    let Some(file) = facts
        .files
        .iter()
        .find(|file| file.path == metric.path || root.join(&file.path) == metric.path)
    else {
        return false;
    };

    if file.language.as_deref() != Some("Rust") || !is_rust_facade_filename(&file.path) {
        return false;
    }

    let content = file
        .content
        .clone()
        .or_else(|| std::fs::read_to_string(root.join(&file.path)).ok())
        .or_else(|| std::fs::read_to_string(&file.path).ok());
    let Some(content) = content else {
        return false;
    };

    let mut saw_facade_declaration = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with("//")
            || trimmed.starts_with("#[")
            || trimmed.starts_with("//!")
            || trimmed.starts_with("///")
        {
            continue;
        }

        if is_rust_facade_declaration(trimmed) {
            saw_facade_declaration = true;
            continue;
        }

        return false;
    }

    saw_facade_declaration
}

fn is_rust_facade_filename(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| matches!(name, "lib.rs" | "mod.rs"))
}

fn is_rust_facade_declaration(line: &str) -> bool {
    let line = line.strip_suffix(';').unwrap_or(line).trim();
    let line = line
        .strip_prefix("pub(crate) ")
        .or_else(|| line.strip_prefix("pub(super) "))
        .or_else(|| line.strip_prefix("pub "))
        .unwrap_or(line);

    line.starts_with("mod ")
        || line.starts_with("use ")
        || line.starts_with("extern crate ")
        || line.starts_with("pub use ")
}

fn relative_path(path: &Path, root: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

/// Re-encodes a file-level coupling graph as a graph v2 `GraphSnapshot` so the
/// shared v2 SCC cycle detection can run over it, returning the snapshot and a
/// map back to file paths. Only file nodes and dependency (`Imports`) edges are
/// produced. Kept local to this rule for now; it can move into a shared scan
/// stage once one computes a snapshot directly.
fn coupling_graph_snapshot(
    graph: &CouplingGraph,
) -> (GraphSnapshot, BTreeMap<GraphNodeId, PathBuf>) {
    let node_id = |path: &Path| GraphNodeId::new(format!("file:{}", path.display()));
    let mut snapshot = GraphSnapshot::new();
    let mut path_by_id = BTreeMap::new();

    for path in &graph.nodes {
        let id = node_id(path);
        path_by_id.insert(id.clone(), path.clone());
        snapshot.add_node(GraphNode {
            id,
            kind: GraphNodeKind::File,
            label: path.display().to_string(),
            path: Some(path.clone()),
        });
    }

    for (from, targets) in &graph.edges {
        for to in targets {
            snapshot.add_edge(GraphEdge {
                from: node_id(from),
                to: node_id(to),
                kind: GraphEdgeKind::Imports,
                provenance: GraphEdgeProvenance::Import,
                confidence: GraphEdgeConfidence::High,
            });
        }
    }

    (snapshot, path_by_id)
}
