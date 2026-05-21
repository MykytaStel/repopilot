use crate::audits::context::{FileRole, classify_file};
use crate::findings::types::Finding;
use crate::graph::{CouplingGraph, build_coupling_graph, compute_metrics, detect_cycles};
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
pub const CONTEXT_GRAPH_SCHEMA_VERSION: u32 = 1;
pub const CONTEXT_GRAPH_RESOLVER_VERSION: &str = "context-graph-v1";

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
