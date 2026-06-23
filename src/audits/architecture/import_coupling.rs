use crate::findings::types::{Confidence, Evidence, Finding, FindingCategory, Severity};
use crate::graph::{
    CouplingGraph, FileMetrics, ImportResolutionStats, build_coupling_graph_with_resolution,
    coupling_file_metrics,
};
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

mod cycles;
mod rust_facade;

#[cfg(test)]
mod tests;

use rust_facade::is_pure_rust_facade;

pub struct ImportCouplingAudit;

impl ImportCouplingAudit {
    pub fn audit_with_graph(
        &self,
        facts: &ScanFacts,
        config: &ScanConfig,
        root: &Path,
    ) -> (Vec<Finding>, CouplingGraph, ImportResolutionStats) {
        let (graph, resolution) = build_coupling_graph_with_resolution(facts, root);
        let metrics = coupling_file_metrics(&graph);

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
                    &resolution,
                ));
            }
        }

        cycles::emit_circular_dependency_findings(&graph, &prod_files, root, &mut findings);

        (findings, graph, resolution)
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
    resolution: &ImportResolutionStats,
) -> Finding {
    let path = relative_path(&metric.path, root);

    let mut snippet = format!(
        "{} fan_in={}, fan_out={}, instability={}%; thresholds: fan_in>={min_fan_in}, instability>={min_instability_pct}%.",
        path.display(),
        metric.fan_in,
        metric.fan_out,
        instability_pct
    );

    // Instability is derived from fan-in; every unresolved relative import in
    // the repository is a potential missing importer of this file, so the
    // measured fan-in is a lower bound and the instability claim is weaker.
    let confidence = if resolution.is_empty() {
        Confidence::High
    } else {
        snippet.push_str(&format!(
            "\n{} unresolved relative import(s) in the repository — fan-in may be undercounted.",
            resolution.total()
        ));
        Confidence::Medium
    };

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
        confidence,
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

fn relative_path(path: &Path, root: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}
