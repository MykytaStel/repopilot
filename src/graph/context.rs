use crate::audits::context::{FileRole, classify_file};
use crate::findings::types::Finding;
use crate::graph::{
    CouplingGraph, build_coupling_graph, compute_metrics, detect_cycles_bounded,
    without_rust_module_containment_edges,
};
use crate::review::diff::{ChangeStatus, ChangedFile};
use crate::risk::RiskPriority;
use crate::scan::facts::{FileFacts, ScanFacts};
pub use crate::scan::types::{
    ContextGraphCacheInfo, ContextGraphFileMetric, ContextGraphSummary, ContextRiskCluster,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const CONTEXT_GRAPH_CACHE_NAME: &str = "repo_context.json";
pub const CONTEXT_GRAPH_SCHEMA_VERSION: u32 = 3;
pub const CONTEXT_GRAPH_RESOLVER_VERSION: &str = "context-graph-v2";
pub const MAX_CONTEXT_GRAPH_CYCLES: usize = 20;
pub const MAX_CONTEXT_GRAPH_METRICS: usize = 10;
pub const MAX_CONTEXT_GRAPH_RISKY_CLUSTERS: usize = 20;
pub const MAX_CONTEXT_GRAPH_BLAST_RADIUS: usize = 50;

/// Repository-level graph/context metadata used for import impact analysis.
///
/// This cache is intentionally not a full `ScanFacts` or source-content cache:
/// it preserves file context, imports, detected frameworks, and derived graph
/// edges. Callers must still scan files when they need source text, file-audit
/// inputs, or the authoritative current finding set.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepoContextGraph {
    pub root_path: PathBuf,
    pub nodes: Vec<RepoContextNode>,
    pub edges: BTreeMap<PathBuf, BTreeSet<PathBuf>>,
    #[serde(default)]
    pub detected_frameworks: Vec<crate::frameworks::DetectedFramework>,
    #[serde(default)]
    pub framework_projects: Vec<crate::frameworks::FrameworkProject>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub react_native: Option<crate::frameworks::ReactNativeArchitectureProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepoContextNode {
    pub path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub frameworks: Vec<String>,
    #[serde(default)]
    pub runtimes: Vec<String>,
    #[serde(default)]
    pub paradigms: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_package: Option<String>,
    pub non_empty_lines: usize,
    #[serde(default)]
    pub imports: Vec<String>,
    pub is_test: bool,
    pub is_generated: bool,
    pub is_config: bool,
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CachedRepoContextGraph {
    pub schema_version: u32,
    pub repopilot_version: String,
    pub config_fingerprint: String,
    pub resolver_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository_fingerprint: Option<RepositoryFingerprint>,
    pub input_fingerprint: String,
    pub graph_fingerprint: String,
    pub graph: RepoContextGraph,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepositoryFingerprint {
    pub head_oid: String,
    pub head_tree_oid: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
}

pub struct RepoContextGraphLoad {
    pub graph: RepoContextGraph,
    pub cache_info: ContextGraphCacheInfo,
}

mod cache;
mod graph_impl;
mod summary;

pub use cache::{
    context_graph_cache_miss, context_graph_cache_path, load_repo_context_graph,
    write_repo_context_graph,
};
pub use summary::summarize_context_graph;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
    use crate::risk::{RiskAssessment, priority_for_score};

    #[test]
    fn summary_caps_cycles_and_marks_truncation() {
        let mut nodes = Vec::new();
        let mut edges = BTreeMap::new();
        for index in 0..(MAX_CONTEXT_GRAPH_CYCLES + 3) {
            let left = PathBuf::from(format!("src/cycle_{index}_a.rs"));
            let right = PathBuf::from(format!("src/cycle_{index}_b.rs"));
            nodes.push(node(&left));
            nodes.push(node(&right));
            edges
                .entry(left.clone())
                .or_insert_with(BTreeSet::new)
                .insert(right.clone());
            edges
                .entry(right)
                .or_insert_with(BTreeSet::new)
                .insert(left);
        }

        let graph = RepoContextGraph {
            root_path: PathBuf::from("."),
            nodes,
            edges,
            detected_frameworks: Vec::new(),
            framework_projects: Vec::new(),
            react_native: None,
        };

        let summary = summarize_context_graph(&graph, &[], &[]);

        assert_eq!(summary.cycles.len(), MAX_CONTEXT_GRAPH_CYCLES);
        assert!(summary.truncated.iter().any(|value| value == "cycles"));
    }

    #[test]
    fn summary_caps_risky_clusters_and_marks_truncation() {
        let findings = (0..(MAX_CONTEXT_GRAPH_RISKY_CLUSTERS + 3))
            .map(|index| {
                finding(
                    &format!("test.rule-{index}"),
                    &format!("src/area_{index}/file.rs"),
                    80,
                )
            })
            .collect::<Vec<_>>();
        let graph = RepoContextGraph {
            root_path: PathBuf::from("."),
            nodes: Vec::new(),
            edges: BTreeMap::new(),
            detected_frameworks: Vec::new(),
            framework_projects: Vec::new(),
            react_native: None,
        };

        let summary = summarize_context_graph(&graph, &findings, &[]);

        assert_eq!(
            summary.risky_clusters.len(),
            MAX_CONTEXT_GRAPH_RISKY_CLUSTERS
        );
        assert!(
            summary
                .truncated
                .iter()
                .any(|value| value == "risky_clusters")
        );
    }

    fn node(path: &Path) -> RepoContextNode {
        RepoContextNode {
            path: path.to_path_buf(),
            language: Some("Rust".to_string()),
            roles: Vec::new(),
            frameworks: Vec::new(),
            runtimes: Vec::new(),
            paradigms: Vec::new(),
            workspace_package: None,
            non_empty_lines: 1,
            imports: Vec::new(),
            is_test: false,
            is_generated: false,
            is_config: false,
        }
    }

    fn finding(rule_id: &str, path: &str, score: u8) -> Finding {
        Finding {
            id: String::new(),
            rule_id: rule_id.to_string(),
            title: String::new(),
            description: String::new(),
            recommendation: String::new(),
            category: FindingCategory::Architecture,
            severity: Severity::High,
            confidence: Default::default(),
            evidence: vec![Evidence {
                path: PathBuf::from(path),
                line_start: 1,
                line_end: None,
                snippet: String::new(),
            }],
            workspace_package: None,
            docs_url: None,
            provenance: Default::default(),
            risk: RiskAssessment {
                score,
                priority: priority_for_score(score),
                ..RiskAssessment::default()
            },
        }
    }
}
