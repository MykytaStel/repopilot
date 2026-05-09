use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::graph::{
    CouplingGraph, FileMetrics, build_coupling_graph, compute_metrics, detect_cycles,
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
        let cycles = detect_cycles(&graph);

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

        findings.sort_by(|left, right| {
            finding_path(left)
                .cmp(finding_path(right))
                .then_with(|| left.rule_id.cmp(&right.rule_id))
                .then_with(|| left.title.cmp(&right.title))
        });

        (findings, graph)
    }
}

fn excessive_fan_out_finding(metric: &FileMetrics, root: &Path, threshold: usize) -> Finding {
    let path = relative_path(&metric.path, root);

    Finding {
        id: String::new(),
        rule_id: "architecture.excessive-fan-out".to_string(),
        title: "File imports too many project files".to_string(),
        description: format!(
            "This file imports {} project files, exceeding the configured fan-out threshold of {threshold}.",
            metric.fan_out
        ),
        category: FindingCategory::Architecture,
        severity: Severity::Medium,
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
        title: "Highly unstable import hub".to_string(),
        description: format!(
            "This file is imported by {} files while also importing {} files, making it a highly unstable hub.",
            metric.fan_in, metric.fan_out
        ),
        category: FindingCategory::Architecture,
        severity: Severity::High,
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
    }
}

fn circular_dependency_finding(cycle: &[PathBuf], root: &Path) -> Finding {
    let relative_cycle: Vec<PathBuf> = cycle.iter().map(|path| relative_path(path, root)).collect();
    let cycle_path = relative_cycle
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(" -> ");
    let evidence_path = relative_cycle
        .first()
        .cloned()
        .unwrap_or_else(|| PathBuf::from("."));
    let file_count = relative_cycle.len();

    Finding {
        id: String::new(),
        rule_id: "architecture.circular-dependency".to_string(),
        title: "Circular dependency detected".to_string(),
        description: format!(
            "A circular dependency was detected across {file_count} project files."
        ),
        category: FindingCategory::Architecture,
        severity: Severity::High,
        evidence: vec![Evidence {
            path: evidence_path,
            line_start: 1,
            line_end: None,
            snippet: format!("Cycle ({file_count} files): {cycle_path}."),
        }],
        workspace_package: None,
    }
}

fn relative_path(path: &Path, root: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

fn finding_path(finding: &Finding) -> &Path {
    finding
        .evidence
        .first()
        .map(|evidence| evidence.path.as_path())
        .unwrap_or_else(|| Path::new(""))
}
