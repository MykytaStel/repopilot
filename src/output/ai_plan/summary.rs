fn render_summary(out: &mut String, findings: &[&Finding]) {
    let critical = findings
        .iter()
        .filter(|finding| finding.severity == Severity::Critical)
        .count();
    let high = findings
        .iter()
        .filter(|finding| finding.severity == Severity::High)
        .count();
    let medium = findings
        .iter()
        .filter(|finding| finding.severity == Severity::Medium)
        .count();
    let low = findings
        .iter()
        .filter(|finding| finding.severity == Severity::Low)
        .count();
    let _ = writeln!(
        out,
        "## Priority Summary\n\n- Total: {} findings\n- Critical: {critical}\n- High: {high}\n- Medium: {medium}\n- Low: {low}",
        findings.len()
    );
}

fn render_context_graph_plan(out: &mut String, graph: Option<&ContextGraphSummary>) {
    let Some(graph) = graph.filter(|graph| graph.files > 0) else {
        return;
    };

    let _ = writeln!(out, "\n## Context Risk Graph\n");
    let _ = writeln!(
        out,
        "- Scope: {} files, {} import edges",
        graph.files, graph.import_edges
    );

    let _ = writeln!(out, "\n### Edit Order");
    if graph.risky_clusters.is_empty() && graph.top_dependencies.is_empty() {
        let _ = writeln!(
            out,
            "- Start with the changed files and keep edits isolated."
        );
    } else {
        for cluster in graph.risky_clusters.iter().take(5) {
            let _ = writeln!(
                out,
                "- `{}` in `{}`: {} finding(s), max risk {}",
                cluster.rule_id, cluster.scope, cluster.count, cluster.max_score
            );
        }
        for file in graph.top_dependencies.iter().take(3) {
            let _ = writeln!(
                out,
                "- Edit shared dependency `{}` before callers.",
                file.path.display()
            );
        }
    }

    render_graph_file_list(out, "Blast Radius", &graph.changed_blast_radius);
    render_graph_metrics(out, "High-Context Files", graph);

    let _ = writeln!(out, "\n### Verification Focus");
    if !graph.changed_blast_radius.is_empty() {
        let _ = writeln!(out, "- Run tests covering blast-radius files.");
    }
    if !graph.cycles.is_empty() {
        let _ = writeln!(
            out,
            "- Re-check import cycles after edits ({} detected).",
            graph.cycles.len()
        );
    }
    if !graph.risky_clusters.is_empty() {
        let _ = writeln!(out, "- Re-run the scan and verify risky clusters shrink.");
    }
    if graph.changed_blast_radius.is_empty()
        && graph.cycles.is_empty()
        && graph.risky_clusters.is_empty()
    {
        let _ = writeln!(out, "- Use the focused scan and review commands below.");
    }
}

fn render_graph_file_list(out: &mut String, title: &str, files: &[std::path::PathBuf]) {
    let _ = writeln!(out, "\n### {title}");
    if files.is_empty() {
        let _ = writeln!(out, "- No import-based downstream files detected.");
        return;
    }
    for path in files.iter().take(10) {
        let _ = writeln!(out, "- `{}`", path.display());
    }
}

fn render_graph_metrics(out: &mut String, title: &str, graph: &ContextGraphSummary) {
    let _ = writeln!(out, "\n### {title}");
    let mut rendered = 0usize;
    for metric in graph.top_dependencies.iter().take(5) {
        render_graph_metric(out, "dependency", metric);
        rendered += 1;
    }
    for metric in graph.top_hubs.iter().take(5usize.saturating_sub(rendered)) {
        render_graph_metric(out, "hub", metric);
    }
    if rendered == 0 && graph.top_hubs.is_empty() {
        let _ = writeln!(out, "- No shared dependency or hub files detected.");
    }
}

fn render_graph_metric(out: &mut String, label: &str, metric: &ContextGraphFileMetric) {
    let _ = writeln!(
        out,
        "- `{}`: {label}, fan-in {}, fan-out {}, instability {:.2}",
        metric.path.display(),
        metric.fan_in,
        metric.fan_out,
        metric.instability
    );
}
