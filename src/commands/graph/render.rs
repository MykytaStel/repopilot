//! Rendering for `inspect graph`: console, Markdown, Graphviz DOT, and Mermaid
//! views of the context risk graph. The command entry point and JSON DTO live
//! in the parent `graph` module.

use repopilot::graph::context::{ContextGraphFileMetric, ContextGraphSummary};
use repopilot::scan::types::ScanSummary;
use std::fmt::Write as FmtWrite;

pub(super) fn render_console(summary: &ScanSummary) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "RepoPilot Context Risk Graph\n");
    let _ = writeln!(output, "Path: {}", summary.root_path.display());
    render_cache_console(&mut output, summary);

    let Some(graph) = &summary.artifacts.context_graph_summary else {
        output.push_str("No context graph summary available.\n");
        return output;
    };

    let _ = writeln!(
        output,
        "Graph: {} files, {} import edges",
        graph.files, graph.import_edges
    );
    render_truncation_console(&mut output, graph);
    render_metrics_console(&mut output, "Top dependencies", &graph.top_dependencies);
    render_metrics_console(&mut output, "Top hubs", &graph.top_hubs);
    render_blast_radius_console(&mut output, graph);
    render_cycles_console(&mut output, graph);
    render_risky_clusters_console(&mut output, graph);
    output
}

pub(super) fn render_markdown(summary: &ScanSummary) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "# RepoPilot Context Risk Graph\n");
    let _ = writeln!(output, "- **Path:** `{}`", summary.root_path.display());
    render_cache_markdown(&mut output, summary);

    let Some(graph) = &summary.artifacts.context_graph_summary else {
        output.push_str("\nNo context graph summary available.\n");
        return output;
    };

    let _ = writeln!(
        output,
        "- **Graph:** {} files, {} import edges",
        graph.files, graph.import_edges
    );
    render_truncation_markdown(&mut output, graph);
    render_metrics_markdown(&mut output, "Top Dependencies", &graph.top_dependencies);
    render_metrics_markdown(&mut output, "Top Hubs", &graph.top_hubs);
    render_blast_radius_markdown(&mut output, graph);
    render_cycles_markdown(&mut output, graph);
    render_risky_clusters_markdown(&mut output, graph);
    output
}

fn render_cache_console(output: &mut String, summary: &ScanSummary) {
    if let Some(cache) = &summary.artifacts.context_graph_cache {
        let _ = writeln!(
            output,
            "Cache: {} ({}) at {}",
            cache.status,
            cache.reason,
            cache.cache_path.display()
        );
    }
}

fn render_cache_markdown(output: &mut String, summary: &ScanSummary) {
    if let Some(cache) = &summary.artifacts.context_graph_cache {
        let _ = writeln!(
            output,
            "- **Cache:** `{}` ({}) at `{}`",
            cache.status,
            cache.reason,
            cache.cache_path.display()
        );
    }
}

fn render_truncation_console(output: &mut String, graph: &ContextGraphSummary) {
    if graph.truncated.is_empty() {
        return;
    }
    let _ = writeln!(output, "Truncated: {}", graph.truncated.join(", "));
}

fn render_truncation_markdown(output: &mut String, graph: &ContextGraphSummary) {
    if graph.truncated.is_empty() {
        return;
    }
    let _ = writeln!(
        output,
        "- **Truncated:** `{}`",
        graph.truncated.join("`, `")
    );
}

fn render_metrics_console(output: &mut String, title: &str, metrics: &[ContextGraphFileMetric]) {
    let _ = writeln!(output, "\n{title}:");
    if metrics.is_empty() {
        output.push_str("  none\n");
        return;
    }
    for metric in metrics {
        let _ = writeln!(
            output,
            "  {} fan-in={} fan-out={} instability={:.2}",
            metric.path.display(),
            metric.fan_in,
            metric.fan_out,
            metric.instability
        );
    }
}

fn render_metrics_markdown(output: &mut String, title: &str, metrics: &[ContextGraphFileMetric]) {
    let _ = writeln!(output, "\n## {title}\n");
    if metrics.is_empty() {
        output.push_str("No files detected.\n");
        return;
    }
    output.push_str("| File | Fan-in | Fan-out | Instability | Roles |\n");
    output.push_str("| --- | ---: | ---: | ---: | --- |\n");
    for metric in metrics {
        let roles = if metric.roles.is_empty() {
            "n/a".to_string()
        } else {
            metric.roles.join(", ")
        };
        let _ = writeln!(
            output,
            "| `{}` | {} | {} | {:.2} | {} |",
            metric.path.display(),
            metric.fan_in,
            metric.fan_out,
            metric.instability,
            roles
        );
    }
}

fn render_blast_radius_console(output: &mut String, graph: &ContextGraphSummary) {
    output.push_str("\nBlast radius:\n");
    if graph.changed_blast_radius.is_empty() {
        output.push_str("  none\n");
        return;
    }
    for path in &graph.changed_blast_radius {
        let _ = writeln!(output, "  {}", path.display());
    }
}

fn render_blast_radius_markdown(output: &mut String, graph: &ContextGraphSummary) {
    output.push_str("\n## Blast Radius\n\n");
    if graph.changed_blast_radius.is_empty() {
        output.push_str("No changed-file blast radius in this full graph inspection.\n");
        return;
    }
    for path in &graph.changed_blast_radius {
        let _ = writeln!(output, "- `{}`", path.display());
    }
}

fn render_cycles_console(output: &mut String, graph: &ContextGraphSummary) {
    let _ = writeln!(output, "\nCycles: {}", graph.cycles.len());
    for cycle in &graph.cycles {
        let rendered = cycle
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(" -> ");
        let _ = writeln!(output, "  {rendered}");
    }
}

fn render_cycles_markdown(output: &mut String, graph: &ContextGraphSummary) {
    output.push_str("\n## Cycles\n\n");
    if graph.cycles.is_empty() {
        output.push_str("No import cycles detected.\n");
        return;
    }
    for cycle in &graph.cycles {
        let rendered = cycle
            .iter()
            .map(|path| format!("`{}`", path.display()))
            .collect::<Vec<_>>()
            .join(" -> ");
        let _ = writeln!(output, "- {rendered}");
    }
}

fn render_risky_clusters_console(output: &mut String, graph: &ContextGraphSummary) {
    output.push_str("\nRisky clusters:\n");
    if graph.risky_clusters.is_empty() {
        output.push_str("  none\n");
        return;
    }
    for cluster in &graph.risky_clusters {
        let _ = writeln!(
            output,
            "  {} in {}: {} finding(s), max risk {}, {}",
            cluster.rule_id,
            cluster.scope,
            cluster.count,
            cluster.max_score,
            cluster.priority.label()
        );
    }
}

fn render_risky_clusters_markdown(output: &mut String, graph: &ContextGraphSummary) {
    output.push_str("\n## Risky Clusters\n\n");
    if graph.risky_clusters.is_empty() {
        output.push_str("No risky clusters detected.\n");
        return;
    }
    output.push_str("| Rule | Scope | Count | Max risk | Priority |\n");
    output.push_str("| --- | --- | ---: | ---: | --- |\n");
    for cluster in &graph.risky_clusters {
        let _ = writeln!(
            output,
            "| `{}` | `{}` | {} | {} | {} |",
            cluster.rule_id,
            cluster.scope,
            cluster.count,
            cluster.max_score,
            cluster.priority.label()
        );
    }
}

pub(super) fn render_dot(summary: &ScanSummary) -> String {
    let mut out = String::new();
    out.push_str("digraph {\n");
    if let Some(graph) = &summary.artifacts.coupling_graph {
        // Output all nodes first to ensure isolated nodes are rendered
        for node in &graph.nodes {
            let rel_path = node.strip_prefix(&summary.root_path).unwrap_or(node);
            let _ = writeln!(out, "  {:?};", rel_path.to_string_lossy());
        }
        // Output edges
        for (source, targets) in &graph.edges {
            let rel_source = source.strip_prefix(&summary.root_path).unwrap_or(source);
            let source_str = rel_source.to_string_lossy();
            for target in targets {
                let rel_target = target.strip_prefix(&summary.root_path).unwrap_or(target);
                let target_str = rel_target.to_string_lossy();
                let _ = writeln!(out, "  {:?} -> {:?};", source_str, target_str);
            }
        }
    }
    out.push_str("}\n");
    out
}

pub(super) fn render_mermaid(summary: &ScanSummary) -> String {
    let mut out = String::new();
    out.push_str("graph TD\n");
    if let Some(graph) = &summary.artifacts.coupling_graph {
        let mut node_ids = std::collections::HashMap::new();
        // Register and print nodes
        for (id_counter, node) in graph.nodes.iter().enumerate() {
            let rel_path = node.strip_prefix(&summary.root_path).unwrap_or(node);
            let path_str = rel_path.to_string_lossy().replace('"', "\\\"");
            let id = format!("n{}", id_counter);
            node_ids.insert(node.clone(), id.clone());
            let _ = writeln!(out, "  {}[\"{}\"]", id, path_str);
        }
        // Output edges
        for (source, targets) in &graph.edges {
            if let Some(source_id) = node_ids.get(source) {
                for target in targets {
                    if let Some(target_id) = node_ids.get(target) {
                        let _ = writeln!(out, "  {} --> {}", source_id, target_id);
                    }
                }
            }
        }
    }
    out
}
