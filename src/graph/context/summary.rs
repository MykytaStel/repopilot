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
