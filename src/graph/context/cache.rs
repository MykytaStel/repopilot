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
        "knowledge_pack": stable_hash_hex(include_bytes!("../../knowledge/packs/core.toml")),
        "nodes": &graph.nodes,
        "edges": &graph.edges,
        "frameworks": &graph.detected_frameworks,
        "framework_projects": &graph.framework_projects,
        "react_native": &graph.react_native,
    });
    stable_hash_hex(input.to_string().as_bytes())
}
