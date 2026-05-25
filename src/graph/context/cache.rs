use super::*;
use std::process::Command;

pub fn load_repo_context_graph(root: &Path, config: &ScanConfig) -> Option<RepoContextGraphLoad> {
    let cache_path = context_graph_cache_path(root);
    let cached = read_cached_repo_context_graph(&cache_path)?;
    if !valid_cached_graph(&cached, root, config) {
        return None;
    }

    // This cache stores graph/context metadata only. Changed scans must still
    // analyze changed file contents and patch this graph before scoring.
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
    let (input_fingerprint, graph_fingerprint) = context_graph_fingerprints(graph);
    let repository_fingerprint = repository_fingerprint(root);

    if let Some(cached) = read_cached_repo_context_graph(&cache_path)
        && valid_cached_graph_metadata(&cached, root, config)
        && cached.input_fingerprint == input_fingerprint
        && cached.graph_fingerprint == graph_fingerprint
    {
        return Ok(ContextGraphCacheInfo {
            status: "hit".to_string(),
            reason: "valid-context-graph-cache".to_string(),
            cache_path,
        });
    }

    let cached = CachedRepoContextGraph {
        schema_version: CONTEXT_GRAPH_SCHEMA_VERSION,
        repopilot_version: env!("CARGO_PKG_VERSION").to_string(),
        config_fingerprint: config_fingerprint(config),
        resolver_version: CONTEXT_GRAPH_RESOLVER_VERSION.to_string(),
        repository_fingerprint,
        input_fingerprint,
        graph_fingerprint,
        graph: graph.clone(),
    };
    let rendered = serde_json::to_vec(&cached).map_err(io::Error::other)?;
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

fn valid_cached_graph(cached: &CachedRepoContextGraph, root: &Path, config: &ScanConfig) -> bool {
    if !valid_cached_graph_metadata(cached, root, config) {
        return false;
    }

    let (input_fingerprint, graph_fingerprint) = context_graph_fingerprints(&cached.graph);
    cached.input_fingerprint == input_fingerprint && cached.graph_fingerprint == graph_fingerprint
}

fn valid_cached_graph_metadata(
    cached: &CachedRepoContextGraph,
    root: &Path,
    config: &ScanConfig,
) -> bool {
    cached.schema_version == CONTEXT_GRAPH_SCHEMA_VERSION
        && cached.repopilot_version == env!("CARGO_PKG_VERSION")
        && cached.config_fingerprint == config_fingerprint(config)
        && cached.resolver_version == CONTEXT_GRAPH_RESOLVER_VERSION
        && cached.repository_fingerprint == repository_fingerprint(root)
}

pub fn context_graph_cache_path(root: &Path) -> PathBuf {
    cache_dir(root).join(CONTEXT_GRAPH_CACHE_NAME)
}

fn context_graph_fingerprints(graph: &RepoContextGraph) -> (String, String) {
    let nodes = stable_node_inputs(graph);
    let edges = stable_edge_inputs(graph);
    let input = serde_json::json!({
        "schema": CONTEXT_GRAPH_SCHEMA_VERSION,
        "resolver": CONTEXT_GRAPH_RESOLVER_VERSION,
        "nodes": &nodes,
        "edges": &edges,
        "frameworks": &graph.detected_frameworks,
        "framework_projects": &graph.framework_projects,
        "react_native": &graph.react_native,
    });
    let input_fingerprint = stable_hash_hex(input.to_string().as_bytes());
    let graph_input = serde_json::json!({
        "schema": CONTEXT_GRAPH_SCHEMA_VERSION,
        "resolver": CONTEXT_GRAPH_RESOLVER_VERSION,
        "risk_formula": crate::risk::FORMULA_VERSION,
        "knowledge_pack": stable_hash_hex(include_bytes!("../../knowledge/packs/core.toml")),
        "input": input_fingerprint,
        "nodes": &nodes,
        "edges": &edges,
        "frameworks": &graph.detected_frameworks,
        "framework_projects": &graph.framework_projects,
        "react_native": &graph.react_native,
    });
    (
        input_fingerprint,
        stable_hash_hex(graph_input.to_string().as_bytes()),
    )
}

fn read_cached_repo_context_graph(path: &Path) -> Option<CachedRepoContextGraph> {
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str::<CachedRepoContextGraph>(&content).ok()
}

fn repository_fingerprint(root: &Path) -> Option<RepositoryFingerprint> {
    let head_oid = git_output(root, &["rev-parse", "HEAD"])?;
    let head_tree_oid = git_output(root, &["rev-parse", "HEAD^{tree}"])?;
    let branch = git_output(root, &["branch", "--show-current"]).filter(|value| !value.is_empty());

    Some(RepositoryFingerprint {
        head_oid,
        head_tree_oid,
        branch,
    })
}

fn git_output(root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .ok()?;

    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn stable_node_inputs(graph: &RepoContextGraph) -> Vec<serde_json::Value> {
    let mut nodes = graph
        .nodes
        .iter()
        .map(|node| {
            serde_json::json!({
                "path": stable_path(&node.path),
                "language": &node.language,
                "roles": sorted_strings(&node.roles),
                "frameworks": sorted_strings(&node.frameworks),
                "runtimes": sorted_strings(&node.runtimes),
                "paradigms": sorted_strings(&node.paradigms),
                "workspace_package": &node.workspace_package,
                "non_empty_lines": node.non_empty_lines,
                "imports": sorted_strings(&node.imports),
                "is_test": node.is_test,
                "is_generated": node.is_generated,
                "is_config": node.is_config,
            })
        })
        .collect::<Vec<_>>();
    nodes.sort_by_key(|value| value["path"].as_str().unwrap_or_default().to_string());
    nodes
}

fn stable_edge_inputs(graph: &RepoContextGraph) -> Vec<serde_json::Value> {
    graph
        .edges
        .iter()
        .map(|(source, targets)| {
            let mut targets = targets
                .iter()
                .map(|path| stable_path(path))
                .collect::<Vec<_>>();
            targets.sort();
            serde_json::json!({
                "source": stable_path(source),
                "targets": targets,
            })
        })
        .collect()
}

fn stable_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn sorted_strings(values: &[String]) -> Vec<String> {
    let mut values = values.to_vec();
    values.sort();
    values
}
