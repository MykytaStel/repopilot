use crate::audits::context::{FileRole, classify_file};
use crate::findings::types::Finding;
use crate::graph::v2::{build_coupling_graph_snapshot, direct_dependents};
use crate::graph::{
    CouplingGraph, coupling_file_metrics, detect_cycles_bounded,
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
pub const CONTEXT_GRAPH_SCHEMA_VERSION: u32 = 4;
pub const CONTEXT_GRAPH_RESOLVER_VERSION: &str = "context-graph-v3";
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
    pub deferred_edges: BTreeMap<PathBuf, BTreeSet<PathBuf>>,
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
    #[serde(default)]
    pub deferred_imports: Vec<String>,
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
mod tests;
