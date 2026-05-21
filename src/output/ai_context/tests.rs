use super::header::risk_level;
use super::{AiContextRenderOptions, AiFocusCategory, render};
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::graph::context::{ContextGraphFileMetric, ContextGraphSummary, ContextRiskCluster};
use crate::risk::RiskPriority;
use crate::scan::types::ScanSummary;
use std::path::PathBuf;

fn make_finding(
    rule_id: &str,
    title: &str,
    severity: Severity,
    category: FindingCategory,
    path: &str,
    line: usize,
) -> Finding {
    Finding {
        id: rule_id.to_string(),
        rule_id: rule_id.to_string(),
        recommendation: Finding::recommendation_for_rule_id(rule_id),
        title: title.to_string(),
        description: format!("Description for {title}"),
        category,
        severity,
        confidence: Default::default(),
        evidence: vec![Evidence {
            path: PathBuf::from(path),
            line_start: line,
            line_end: None,
            snippet: format!("// snippet for {title}"),
        }],
        workspace_package: None,
        docs_url: None,
        provenance: Default::default(),
        risk: Default::default(),
    }
}

fn make_summary(findings: Vec<Finding>) -> ScanSummary {
    ScanSummary {
        root_path: PathBuf::from("/my-project"),
        files_discovered: 0,
        files_analyzed: 42,
        non_empty_lines: 3000,
        directories_count: 10,
        languages: vec![],
        findings,
        ..Default::default()
    }
}

include!("tests/group_1.rs");
include!("tests/group_2.rs");
include!("tests/group_3.rs");

fn context_graph_summary() -> ContextGraphSummary {
    ContextGraphSummary {
        files: 3,
        import_edges: 2,
        top_hubs: Vec::new(),
        top_dependencies: vec![ContextGraphFileMetric {
            path: PathBuf::from("src/core.rs"),
            fan_in: 3,
            fan_out: 1,
            instability: 0.25,
            language: Some("Rust".to_string()),
            roles: vec!["source".to_string()],
        }],
        cycles: Vec::new(),
        changed_blast_radius: vec![PathBuf::from("src/caller.rs")],
        risky_clusters: vec![ContextRiskCluster {
            rule_id: "architecture.large-file".to_string(),
            scope: "src".to_string(),
            count: 2,
            max_score: 60,
            priority: RiskPriority::P2,
        }],
        truncated: Vec::new(),
    }
}
