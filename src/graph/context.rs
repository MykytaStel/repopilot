use crate::audits::context::{FileRole, classify_file};
use crate::findings::types::Finding;
use crate::graph::{CouplingGraph, build_coupling_graph, compute_metrics, detect_cycles_bounded};
use crate::review::diff::{ChangeStatus, ChangedFile};
use crate::risk::RiskPriority;
use crate::scan::cache::{cache_dir, config_fingerprint, relative_cache_path, stable_hash_hex};
use crate::scan::config::ScanConfig;
use crate::scan::facts::{FileFacts, ScanFacts};
use crate::scan::types::ScanDiagnostic;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const CONTEXT_GRAPH_CACHE_NAME: &str = "repo_context.json";
pub const CONTEXT_GRAPH_SCHEMA_VERSION: u32 = 2;
pub const CONTEXT_GRAPH_RESOLVER_VERSION: &str = "context-graph-v1";
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

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextGraphSummary {
    pub files: usize,
    pub import_edges: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub top_hubs: Vec<ContextGraphFileMetric>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub top_dependencies: Vec<ContextGraphFileMetric>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cycles: Vec<Vec<PathBuf>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changed_blast_radius: Vec<PathBuf>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub risky_clusters: Vec<ContextRiskCluster>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub truncated: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextGraphFileMetric {
    pub path: PathBuf,
    pub fan_in: usize,
    pub fan_out: usize,
    pub instability: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<String>,
}

impl Eq for ContextGraphFileMetric {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextRiskCluster {
    pub rule_id: String,
    pub scope: String,
    pub count: usize,
    pub max_score: u8,
    pub priority: RiskPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextGraphCacheInfo {
    pub status: String,
    pub reason: String,
    pub cache_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CachedRepoContextGraph {
    pub schema_version: u32,
    pub repopilot_version: String,
    pub config_fingerprint: String,
    pub resolver_version: String,
    pub input_fingerprint: String,
    pub graph_fingerprint: String,
    pub graph: RepoContextGraph,
}

pub struct RepoContextGraphLoad {
    pub graph: RepoContextGraph,
    pub cache_info: ContextGraphCacheInfo,
}

include!("context/graph_impl.rs");
include!("context/summary.rs");
include!("context/cache.rs");

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
