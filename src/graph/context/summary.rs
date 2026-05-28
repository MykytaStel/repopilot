use super::*;

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
    let all_top_hubs = metrics
        .iter()
        .filter(|metric| metric.fan_out > 0)
        .map(|metric| metric_from_graph(metric, &node_by_path))
        .collect::<Vec<_>>();
    let top_hubs_truncated = all_top_hubs.len() > MAX_CONTEXT_GRAPH_METRICS;
    let top_hubs = all_top_hubs
        .into_iter()
        .take(MAX_CONTEXT_GRAPH_METRICS)
        .collect();

    metrics.sort_by(|left, right| {
        right
            .fan_in
            .cmp(&left.fan_in)
            .then_with(|| right.fan_out.cmp(&left.fan_out))
            .then_with(|| left.path.cmp(&right.path))
    });
    let all_top_dependencies = metrics
        .iter()
        .filter(|metric| metric.fan_in > 0)
        .map(|metric| metric_from_graph(metric, &node_by_path))
        .collect::<Vec<_>>();
    let top_dependencies_truncated = all_top_dependencies.len() > MAX_CONTEXT_GRAPH_METRICS;
    let top_dependencies = all_top_dependencies
        .into_iter()
        .take(MAX_CONTEXT_GRAPH_METRICS)
        .collect();

    let cycle_graph = without_rust_module_containment_edges(&coupling_graph);
    let mut cycles = detect_cycles_bounded(&cycle_graph, MAX_CONTEXT_GRAPH_CYCLES + 1);
    let cycles_truncated = cycles.len() > MAX_CONTEXT_GRAPH_CYCLES;
    cycles.truncate(MAX_CONTEXT_GRAPH_CYCLES);

    let (changed_blast_radius, blast_radius_truncated) =
        changed_blast_radius(&coupling_graph, changed_files);
    let (risky_clusters, risky_clusters_truncated) = risky_clusters(findings);
    let mut truncated = Vec::new();
    if top_hubs_truncated {
        truncated.push("top_hubs".to_string());
    }
    if top_dependencies_truncated {
        truncated.push("top_dependencies".to_string());
    }
    if cycles_truncated {
        truncated.push("cycles".to_string());
    }
    if blast_radius_truncated {
        truncated.push("changed_blast_radius".to_string());
    }
    if risky_clusters_truncated {
        truncated.push("risky_clusters".to_string());
    }

    ContextGraphSummary {
        files: graph.nodes.len(),
        import_edges: graph.edges.values().map(BTreeSet::len).sum(),
        top_hubs,
        top_dependencies,
        cycles,
        changed_blast_radius,
        risky_clusters,
        truncated,
    }
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
    graph: &CouplingGraph,
    changed_files: &[ChangedFile],
) -> (Vec<PathBuf>, bool) {
    if changed_files.is_empty() {
        return (Vec::new(), false);
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
