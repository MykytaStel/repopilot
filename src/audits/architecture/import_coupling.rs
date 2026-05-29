use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::graph::{
    CouplingGraph, FileMetrics, build_coupling_graph, compute_metrics, detect_cycles,
    without_rust_module_containment_edges,
};
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;
use std::collections::BTreeSet;
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
        let cycle_graph = without_rust_module_containment_edges(&graph);
        let cycles = detect_cycles(&cycle_graph);

        let mut findings = Vec::new();

        for metric in &metrics {
            if metric.fan_out > config.max_fan_out {
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

fn relative_path(path: &Path, root: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}
