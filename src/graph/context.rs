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

impl RepoContextGraph {
    pub fn from_scan_facts(facts: &ScanFacts, root: &Path, coupling_graph: CouplingGraph) -> Self {
        Self {
            root_path: root.to_path_buf(),
            nodes: facts
                .files
                .iter()
                .map(|file| RepoContextNode::from_file(file, root))
                .collect(),
            edges: relative_edges(coupling_graph.edges, root),
            detected_frameworks: facts.detected_frameworks.clone(),
            framework_projects: facts.framework_projects.clone(),
            react_native: facts.react_native.clone(),
        }
    }

    pub fn to_scan_facts(&self) -> ScanFacts {
        let files = self
            .nodes
            .iter()
            .map(RepoContextNode::to_file_facts)
            .collect::<Vec<_>>();

        let mut languages = HashMap::new();
        let mut non_empty_lines = 0usize;
        for file in &files {
            non_empty_lines += file.non_empty_lines;
            if let Some(language) = &file.language {
                *languages.entry(language.clone()).or_insert(0usize) += 1;
            }
        }

        ScanFacts {
            root_path: self.root_path.clone(),
            files_discovered: files.len(),
            files_analyzed: files.len(),
            directories_count: directory_count(&files),
            non_empty_lines,
            languages: build_language_summary(languages),
            files,
            detected_frameworks: self.detected_frameworks.clone(),
            framework_projects: self.framework_projects.clone(),
            react_native: self.react_native.clone(),
            ..ScanFacts::default()
        }
    }

    pub fn coupling_graph(&self) -> CouplingGraph {
        let nodes = self
            .nodes
            .iter()
            .map(|node| node.path.clone())
            .chain(
                self.edges
                    .values()
                    .flat_map(|targets| targets.iter().cloned()),
            )
            .collect::<BTreeSet<_>>();

        CouplingGraph {
            edges: self.edges.clone(),
            nodes,
        }
    }

    pub fn apply_changed_facts(
        &mut self,
        repo_root: &Path,
        changed_files: &[ChangedFile],
        patch_files: &[FileFacts],
    ) {
        let removed = changed_files
            .iter()
            .filter(|file| file.status == ChangeStatus::Deleted)
            .map(|file| file.path.clone())
            .collect::<HashSet<_>>();

        self.nodes.retain(|node| !removed.contains(&node.path));
        self.nodes
            .retain(|node| !patch_files.iter().any(|file| file.path == node.path));
        self.nodes.extend(
            patch_files
                .iter()
                .map(|file| RepoContextNode::from_file(file, repo_root)),
        );
        self.nodes.sort_by(|left, right| left.path.cmp(&right.path));

        let mut facts = self.to_scan_facts();
        for file in &mut facts.files {
            if file.path.is_relative() {
                file.path = repo_root.join(&file.path);
            }
        }
        self.edges = relative_edges(build_coupling_graph(&facts, repo_root).edges, repo_root);
    }
}

impl RepoContextNode {
    fn from_file(file: &FileFacts, root: &Path) -> Self {
        let context = classify_file(file);
        let roles = context.role_ids().into_iter().map(str::to_string).collect();
        let frameworks = context
            .framework_ids()
            .into_iter()
            .map(str::to_string)
            .collect();
        let runtimes = context
            .runtime_ids()
            .into_iter()
            .map(str::to_string)
            .collect();
        let paradigms = context
            .paradigm_ids()
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        let is_generated = context.has_role(FileRole::Generated);
        let is_config = context.has_role(FileRole::Config);

        Self {
            path: relative_graph_path(root, &file.path),
            language: file.language.clone(),
            roles,
            frameworks,
            runtimes,
            paradigms,
            workspace_package: None,
            non_empty_lines: file.non_empty_lines,
            imports: file.imports.clone(),
            is_test: context.is_test,
            is_generated,
            is_config,
        }
    }

    fn to_file_facts(&self) -> FileFacts {
        FileFacts {
            path: self.path.clone(),
            language: self.language.clone(),
            non_empty_lines: self.non_empty_lines,
            branch_count: 0,
            imports: self.imports.clone(),
            content: None,
            has_inline_tests: self.is_test,
        }
    }
}

fn relative_edges(
    edges: BTreeMap<PathBuf, BTreeSet<PathBuf>>,
    root: &Path,
) -> BTreeMap<PathBuf, BTreeSet<PathBuf>> {
    edges
        .into_iter()
        .map(|(source, targets)| {
            (
                relative_graph_path(root, &source),
                targets
                    .into_iter()
                    .map(|target| relative_graph_path(root, &target))
                    .collect(),
            )
        })
        .collect()
}

fn relative_graph_path(root: &Path, path: &Path) -> PathBuf {
    PathBuf::from(relative_cache_path(root, path))
}

pub fn summarize_context_graph(
    graph: &RepoContextGraph,
    findings: &[Finding],
    changed_files: &[ChangedFile],
) -> ContextGraphSummary {
    let coupling_graph = graph.coupling_graph();
    let mut metrics = compute_metrics(&coupling_graph);
    let node_by_path = graph
        .nodes
        .iter()
        .map(|node| (node.path.clone(), node))
        .collect::<HashMap<_, _>>();

    metrics.sort_by(|left, right| {
        right
            .fan_out
            .cmp(&left.fan_out)
            .then_with(|| right.fan_in.cmp(&left.fan_in))
            .then_with(|| left.path.cmp(&right.path))
    });
    let top_hubs = metrics
        .iter()
        .filter(|metric| metric.fan_out > 0)
        .take(5)
        .map(|metric| metric_from_graph(metric, &node_by_path))
        .collect();

    metrics.sort_by(|left, right| {
        right
            .fan_in
            .cmp(&left.fan_in)
            .then_with(|| right.fan_out.cmp(&left.fan_out))
            .then_with(|| left.path.cmp(&right.path))
    });
    let top_dependencies = metrics
        .iter()
        .filter(|metric| metric.fan_in > 0)
        .take(5)
        .map(|metric| metric_from_graph(metric, &node_by_path))
        .collect();

    ContextGraphSummary {
        files: graph.nodes.len(),
        import_edges: graph.edges.values().map(BTreeSet::len).sum(),
        top_hubs,
        top_dependencies,
        cycles: detect_cycles(&coupling_graph).into_iter().take(5).collect(),
        changed_blast_radius: changed_blast_radius(&coupling_graph, changed_files),
        risky_clusters: risky_clusters(findings),
    }
}

pub fn load_repo_context_graph(root: &Path, config: &ScanConfig) -> Option<RepoContextGraphLoad> {
    let cache_path = context_graph_cache_path(root);
    let content = fs::read_to_string(&cache_path).ok()?;
    let cached = serde_json::from_str::<CachedRepoContextGraph>(&content).ok()?;
    if !valid_cached_graph(&cached, config) {
        return None;
    }

    Some(RepoContextGraphLoad {
        graph: cached.graph,
        cache_info: ContextGraphCacheInfo {
            status: "hit".to_string(),
            reason: "valid-context-graph-cache".to_string(),
            cache_path,
        },
    })
}

pub fn write_repo_context_graph(
    root: &Path,
    config: &ScanConfig,
    graph: &RepoContextGraph,
) -> io::Result<ContextGraphCacheInfo> {
    let cache_path = context_graph_cache_path(root);
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let cached = CachedRepoContextGraph {
        schema_version: CONTEXT_GRAPH_SCHEMA_VERSION,
        repopilot_version: env!("CARGO_PKG_VERSION").to_string(),
        config_fingerprint: config_fingerprint(config),
        resolver_version: CONTEXT_GRAPH_RESOLVER_VERSION.to_string(),
        graph_fingerprint: context_graph_fingerprint(graph),
        graph: graph.clone(),
    };
    let rendered = serde_json::to_string_pretty(&cached).map_err(io::Error::other)?;
    fs::write(&cache_path, rendered)?;

    Ok(ContextGraphCacheInfo {
        status: "write".to_string(),
        reason: "context-graph-cache-updated".to_string(),
        cache_path,
    })
}

pub fn context_graph_cache_miss(root: &Path, reason: &str) -> ContextGraphCacheInfo {
    ContextGraphCacheInfo {
        status: "miss".to_string(),
        reason: reason.to_string(),
        cache_path: context_graph_cache_path(root),
    }
}

pub fn cache_diagnostic(error: &io::Error) -> ScanDiagnostic {
    ScanDiagnostic::warning(
        "context-graph.cache-write-failed",
        format!("Could not write context graph cache: {error}"),
    )
}

fn valid_cached_graph(cached: &CachedRepoContextGraph, config: &ScanConfig) -> bool {
    cached.schema_version == CONTEXT_GRAPH_SCHEMA_VERSION
        && cached.repopilot_version == env!("CARGO_PKG_VERSION")
        && cached.config_fingerprint == config_fingerprint(config)
        && cached.resolver_version == CONTEXT_GRAPH_RESOLVER_VERSION
        && cached.graph_fingerprint == context_graph_fingerprint(&cached.graph)
}

pub fn context_graph_cache_path(root: &Path) -> PathBuf {
    cache_dir(root).join(CONTEXT_GRAPH_CACHE_NAME)
}

fn context_graph_fingerprint(graph: &RepoContextGraph) -> String {
    let input = serde_json::json!({
        "schema": CONTEXT_GRAPH_SCHEMA_VERSION,
        "resolver": CONTEXT_GRAPH_RESOLVER_VERSION,
        "risk_formula": crate::risk::FORMULA_VERSION,
        "knowledge_pack": stable_hash_hex(include_bytes!("../knowledge/packs/core.toml")),
        "nodes": &graph.nodes,
        "edges": &graph.edges,
        "frameworks": &graph.detected_frameworks,
        "framework_projects": &graph.framework_projects,
        "react_native": &graph.react_native,
    });
    stable_hash_hex(input.to_string().as_bytes())
}

fn metric_from_graph(
    metric: &crate::graph::FileMetrics,
    node_by_path: &HashMap<PathBuf, &RepoContextNode>,
) -> ContextGraphFileMetric {
    let node = node_by_path.get(&metric.path);
    ContextGraphFileMetric {
        path: metric.path.clone(),
        fan_in: metric.fan_in,
        fan_out: metric.fan_out,
        instability: metric.instability,
        language: node.and_then(|node| node.language.clone()),
        roles: node.map(|node| node.roles.clone()).unwrap_or_default(),
    }
}

fn changed_blast_radius(graph: &CouplingGraph, changed_files: &[ChangedFile]) -> Vec<PathBuf> {
    if changed_files.is_empty() {
        return Vec::new();
    }

    let changed = changed_files
        .iter()
        .map(|file| file.path.clone())
        .collect::<HashSet<_>>();
    let mut importers_by_target: BTreeMap<PathBuf, BTreeSet<PathBuf>> = BTreeMap::new();
    for (source, targets) in &graph.edges {
        for target in targets {
            importers_by_target
                .entry(target.clone())
                .or_default()
                .insert(source.clone());
        }
    }

    let mut impacted = BTreeSet::new();
    for path in &changed {
        if let Some(importers) = importers_by_target.get(path) {
            impacted.extend(
                importers
                    .iter()
                    .filter(|importer| !changed.contains(*importer))
                    .cloned(),
            );
        }
    }
    impacted.into_iter().take(20).collect()
}

fn risky_clusters(findings: &[Finding]) -> Vec<ContextRiskCluster> {
    let mut clusters: BTreeMap<(String, String), ContextRiskCluster> = BTreeMap::new();
    for finding in findings {
        let scope = finding
            .evidence
            .first()
            .map(|evidence| cluster_scope(&evidence.path))
            .unwrap_or_else(|| ".".to_string());
        let key = (finding.rule_id.clone(), scope.clone());
        let entry = clusters.entry(key).or_insert_with(|| ContextRiskCluster {
            rule_id: finding.rule_id.clone(),
            scope,
            count: 0,
            max_score: 0,
            priority: RiskPriority::P3,
        });
        entry.count += 1;
        entry.max_score = entry.max_score.max(finding.risk.score);
        if finding.risk.priority.rank() < entry.priority.rank() {
            entry.priority = finding.risk.priority;
        }
    }

    let mut clusters = clusters.into_values().collect::<Vec<_>>();
    clusters.sort_by(|left, right| {
        right
            .max_score
            .cmp(&left.max_score)
            .then_with(|| right.count.cmp(&left.count))
            .then_with(|| left.rule_id.cmp(&right.rule_id))
    });
    clusters.truncate(8);
    clusters
}

fn cluster_scope(path: &Path) -> String {
    let parts = path
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => Some(value.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();

    match (parts.first(), parts.get(1)) {
        (Some(first), Some(second)) if second.contains('.') => first.clone(),
        (Some(first), Some(second)) => format!("{first}/{second}"),
        (Some(first), None) => first.clone(),
        _ => ".".to_string(),
    }
}

fn build_language_summary(
    languages: HashMap<String, usize>,
) -> Vec<crate::scan::types::LanguageSummary> {
    let mut languages = languages
        .into_iter()
        .map(
            |(name, files_analyzed)| crate::scan::types::LanguageSummary {
                name,
                files_analyzed,
            },
        )
        .collect::<Vec<_>>();
    languages.sort_by(|left, right| {
        right
            .files_analyzed
            .cmp(&left.files_analyzed)
            .then_with(|| left.name.cmp(&right.name))
    });
    languages
}

fn directory_count(files: &[FileFacts]) -> usize {
    files
        .iter()
        .filter_map(|file| file.path.parent().map(Path::to_path_buf))
        .collect::<HashSet<_>>()
        .len()
}
