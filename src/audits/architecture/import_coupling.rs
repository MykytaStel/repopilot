use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::graph::v2::{build_coupling_graph_snapshot, find_cycles, shortest_cycle};
use crate::graph::{
    CouplingGraph, FileMetrics, build_coupling_graph, coupling_file_metrics,
    without_rust_module_containment_edges,
};
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

mod rust_facade;

use rust_facade::is_pure_rust_facade;

pub struct ImportCouplingAudit;

impl ImportCouplingAudit {
    pub fn audit_with_graph(
        &self,
        facts: &ScanFacts,
        config: &ScanConfig,
        root: &Path,
    ) -> (Vec<Finding>, CouplingGraph) {
        let graph = build_coupling_graph(facts, root);
        let metrics = coupling_file_metrics(&graph);

        // Circular dependencies now run through graph v2's SCC-based cycle
        // detection. We re-encode the existing (module-containment-stripped)
        // coupling graph as a `GraphSnapshot` via the shared graph adapter and
        // reuse `find_cycles`. The v1 coupling graph is still built and returned
        // to the scan pipeline. Each cycle is one strongly-connected component
        // (set of mutually dependent files).
        let cycle_graph = without_rust_module_containment_edges(&graph);
        let (cycle_snapshot, path_by_id) = build_coupling_graph_snapshot(&cycle_graph);
        let cycles = find_cycles(&cycle_snapshot);

        let classifier = crate::analysis::ArchitectureClassifier::new(&config.module_mappings);
        let mut findings = Vec::new();

        let mut prod_files = HashSet::new();
        let mut file_by_path = HashMap::new();
        for file in &facts.files {
            let is_prod =
                classifier.classify(file).file_role == crate::analysis::FileRole::Production;
            if is_prod {
                prod_files.insert(file.path.clone());
                prod_files.insert(root.join(&file.path));
            }
            file_by_path.insert(file.path.clone(), file);
            file_by_path.insert(root.join(&file.path), file);
        }

        for metric in &metrics {
            let is_prod = prod_files.contains(&metric.path);

            if !is_prod {
                continue;
            }

            let file_facts = file_by_path.get(&metric.path).copied();

            if metric.fan_out > config.max_fan_out && !is_pure_rust_facade(metric, facts, root) {
                findings.push(excessive_fan_out_finding(
                    metric,
                    root,
                    config.max_fan_out,
                    file_facts,
                ));
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
                    file_facts,
                ));
            }
        }

        let mut seen_cycles = BTreeSet::new();
        for cycle in &cycles {
            let members: Vec<PathBuf> = cycle
                .node_ids
                .iter()
                .filter_map(|id| path_by_id.get(id).cloned())
                .collect();

            let is_prod_cycle =
                !members.is_empty() && members.iter().all(|path| prod_files.contains(path));

            if !is_prod_cycle {
                continue;
            }

            if seen_cycles.insert(members.clone()) {
                let shortest: Vec<PathBuf> = shortest_cycle(&cycle_snapshot, cycle)
                    .iter()
                    .filter_map(|id| path_by_id.get(id).cloned())
                    .collect();
                findings.push(circular_dependency_finding(&members, &shortest, root));
            }
        }

        (findings, graph)
    }
}

fn excessive_fan_out_finding(
    metric: &FileMetrics,
    root: &Path,
    threshold: usize,
    file_facts: Option<&crate::scan::facts::FileFacts>,
) -> Finding {
    let path = relative_path(&metric.path, root);

    let mut snippet = format!(
        "{} fan_out={}; threshold={threshold}.",
        path.display(),
        metric.fan_out
    );

    if let Some(facts) = file_facts
        && !facts.imports.is_empty()
    {
        snippet.push_str("\nImports:\n");
        snippet.push_str(&facts.imports.join("\n"));
    }

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
            snippet,
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
    file_facts: Option<&crate::scan::facts::FileFacts>,
) -> Finding {
    let path = relative_path(&metric.path, root);

    let mut snippet = format!(
        "{} fan_in={}, fan_out={}, instability={}%; thresholds: fan_in>={min_fan_in}, instability>={min_instability_pct}%.",
        path.display(),
        metric.fan_in,
        metric.fan_out,
        instability_pct
    );

    if let Some(facts) = file_facts
        && !facts.imports.is_empty()
    {
        snippet.push_str("\nImports:\n");
        snippet.push_str(&facts.imports.join("\n"));
    }

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
            snippet,
        }],
        workspace_package: None,
        docs_url: None,
        provenance: Default::default(),
        risk: Default::default(),
    }
}

/// `component` is the full strongly-connected component (all mutually dependent
/// files); `shortest` is the minimal cycle within it, as a closed path
/// (`a -> b -> a`). The finding leads with the actionable minimal cycle and
/// carries the component size as context, instead of repeating the whole
/// component into every evidence snippet.
fn circular_dependency_finding(
    component: &[PathBuf],
    shortest: &[PathBuf],
    root: &Path,
) -> Finding {
    let component_size = component.len();

    // The closed path repeats its first node at the end; the distinct files are
    // everything but that trailing repeat.
    let closed: Vec<PathBuf> = if shortest.len() >= 2 {
        shortest
            .iter()
            .map(|path| relative_path(path, root))
            .collect()
    } else {
        component
            .iter()
            .map(|path| relative_path(path, root))
            .collect()
    };
    let cycle_path = closed
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(" -> ");
    let distinct: Vec<&PathBuf> = closed.iter().take(closed.len().saturating_sub(1)).collect();
    let distinct = if distinct.is_empty() {
        closed.iter().collect()
    } else {
        distinct
    };

    let context = if component_size > distinct.len() {
        format!(" Part of a strongly-connected component of {component_size} files.")
    } else {
        String::new()
    };

    let evidence = distinct
        .iter()
        .map(|path| Evidence {
            path: (*path).clone(),
            line_start: 1,
            line_end: None,
            snippet: format!("Cycle: {cycle_path}.{context}"),
        })
        .collect();

    Finding {
        id: String::new(),
        rule_id: "architecture.circular-dependency".to_string(),
        recommendation: Finding::recommendation_for_rule_id("architecture.circular-dependency"),
        title: "Circular dependency detected".to_string(),
        description: format!("A circular dependency was detected: {cycle_path}.{context}"),
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

fn relative_path(path: &Path, root: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}
