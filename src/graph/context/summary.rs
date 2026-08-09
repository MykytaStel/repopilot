use super::*;

pub fn summarize_context_graph(
    graph: &RepoContextGraph,
    findings: &[Finding],
    changed_files: &[ChangedFile],
) -> ContextGraphSummary {
    let coupling_graph = graph.coupling_graph();
    let analysis = ContextGraphAnalysis::from_graph(graph);
    let ranked = ranked_metrics(graph, &analysis);
    let (cycles, cycles_truncated) = bounded_cycles(&coupling_graph);
    let dependents = analysis.direct_dependents_by_path();
    let (changed_blast_radius, blast_radius_truncated) =
        changed_blast_radius(&dependents, changed_files);
    let (risky_clusters, risky_clusters_truncated) = risky_clusters(findings);
    let truncated = truncation_labels([
        ("top_hubs", ranked.top_hubs_truncated),
        ("top_dependencies", ranked.top_dependencies_truncated),
        ("cycles", cycles_truncated),
        ("changed_blast_radius", blast_radius_truncated),
        ("risky_clusters", risky_clusters_truncated),
    ]);

    ContextGraphSummary {
        files: graph.nodes.len(),
        import_edges: graph.edges.values().map(BTreeSet::len).sum(),
        top_hubs: ranked.top_hubs,
        top_dependencies: ranked.top_dependencies,
        cycles,
        changed_blast_radius,
        risky_clusters,
        truncated,
    }
}

struct RankedMetrics {
    top_hubs: Vec<ContextGraphFileMetric>,
    top_dependencies: Vec<ContextGraphFileMetric>,
    top_hubs_truncated: bool,
    top_dependencies_truncated: bool,
}

fn ranked_metrics(graph: &RepoContextGraph, analysis: &ContextGraphAnalysis) -> RankedMetrics {
    let mut metrics = analysis.file_metrics();
    debug_assert_eq!(analysis.capabilities().file_nodes, metrics.len());
    let nodes = graph
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
    let hubs = metrics
        .iter()
        .filter(|metric| metric.fan_out > 0)
        .map(|metric| metric_from_graph(metric, &nodes))
        .collect::<Vec<_>>();
    metrics.sort_by(|left, right| {
        right
            .fan_in
            .cmp(&left.fan_in)
            .then_with(|| right.fan_out.cmp(&left.fan_out))
            .then_with(|| left.path.cmp(&right.path))
    });
    let dependencies = metrics
        .iter()
        .filter(|metric| metric.fan_in > 0)
        .map(|metric| metric_from_graph(metric, &nodes))
        .collect::<Vec<_>>();
    RankedMetrics {
        top_hubs_truncated: hubs.len() > MAX_CONTEXT_GRAPH_METRICS,
        top_dependencies_truncated: dependencies.len() > MAX_CONTEXT_GRAPH_METRICS,
        top_hubs: hubs.into_iter().take(MAX_CONTEXT_GRAPH_METRICS).collect(),
        top_dependencies: dependencies
            .into_iter()
            .take(MAX_CONTEXT_GRAPH_METRICS)
            .collect(),
    }
}

fn bounded_cycles(graph: &CouplingGraph) -> (Vec<Vec<PathBuf>>, bool) {
    // The historical path contract excludes deferred and Rust containment edges;
    // graph-v2 SCC membership is not an equivalent public representation.
    let graph = without_rust_module_containment_edges(graph);
    let mut cycles = detect_cycles_bounded(&graph, MAX_CONTEXT_GRAPH_CYCLES + 1);
    let truncated = cycles.len() > MAX_CONTEXT_GRAPH_CYCLES;
    cycles.truncate(MAX_CONTEXT_GRAPH_CYCLES);
    (cycles, truncated)
}

fn truncation_labels<const N: usize>(states: [(&str, bool); N]) -> Vec<String> {
    states
        .into_iter()
        .filter(|(_, truncated)| *truncated)
        .map(|(label, _)| label.to_string())
        .collect()
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

fn changed_blast_radius(
    importers_by_target: &BTreeMap<PathBuf, BTreeSet<PathBuf>>,
    changed_files: &[ChangedFile],
) -> (Vec<PathBuf>, bool) {
    if changed_files.is_empty() {
        return (Vec::new(), false);
    }

    let changed = changed_files
        .iter()
        .map(|file| file.path.clone())
        .collect::<HashSet<_>>();
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
    let truncated = impacted.len() > MAX_CONTEXT_GRAPH_BLAST_RADIUS;
    (
        impacted
            .into_iter()
            .take(MAX_CONTEXT_GRAPH_BLAST_RADIUS)
            .collect(),
        truncated,
    )
}

fn risky_clusters(findings: &[Finding]) -> (Vec<ContextRiskCluster>, bool) {
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
    let truncated = clusters.len() > MAX_CONTEXT_GRAPH_RISKY_CLUSTERS;
    clusters.truncate(MAX_CONTEXT_GRAPH_RISKY_CLUSTERS);
    (clusters, truncated)
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

pub(super) fn build_language_summary(
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

pub(super) fn directory_count(files: &[FileFacts]) -> usize {
    files
        .iter()
        .filter_map(|file| file.path.parent().map(Path::to_path_buf))
        .collect::<HashSet<_>>()
        .len()
}
