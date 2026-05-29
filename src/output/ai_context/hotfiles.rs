use crate::graph::compute_metrics;
use crate::graph::context::{ContextGraphFileMetric, ContextGraphSummary};
use crate::scan::types::ScanSummary;
use std::fmt::Write as FmtWrite;

pub(super) fn render_hot_files(out: &mut String, summary: &ScanSummary) {
    if let Some(graph) = &summary.artifacts.context_graph_summary {
        render_context_risk_graph(out, summary, graph);
        return;
    }

    let graph = match &summary.artifacts.coupling_graph {
        Some(g) => g,
        None => return,
    };

    let mut metrics = compute_metrics(graph);
    metrics.retain(|m| m.fan_in > 0);
    if metrics.is_empty() {
        return;
    }
    metrics.sort_by(|a, b| b.fan_in.cmp(&a.fan_in).then(a.fan_out.cmp(&b.fan_out)));

    let _ = writeln!(out, "## Hot Files (most imported)\n");
    let _ = writeln!(out, "| File | Fan-in | Fan-out | Instability |");
    let _ = writeln!(out, "|---|---|---|---|");
    for m in metrics.iter().take(5) {
        let path = m.path.display().to_string();
        let _ = writeln!(
            out,
            "| {} | {} | {} | {:.2} |",
            path, m.fan_in, m.fan_out, m.instability
        );
    }
    out.push('\n');
}

fn render_context_risk_graph(out: &mut String, summary: &ScanSummary, graph: &ContextGraphSummary) {
    if graph.files == 0 {
        return;
    }

    let _ = writeln!(out, "## Context Risk Graph\n");
    if let Some(cache) = &summary.artifacts.context_graph_cache {
        let _ = writeln!(
            out,
            "- Cache: {} ({})",
            cache.status,
            cache.reason.replace('|', "/")
        );
    }
    let _ = writeln!(
        out,
        "- Graph size: {} files, {} import edges",
        graph.files, graph.import_edges
    );

    render_edit_order(out, graph);
    render_blast_radius(out, graph);
    render_high_context_files(out, graph);
    render_verification_focus(out, graph);
    out.push('\n');
}

fn render_edit_order(out: &mut String, graph: &ContextGraphSummary) {
    let _ = writeln!(out, "\n### Edit Order");
    if graph.risky_clusters.is_empty() && graph.top_dependencies.is_empty() {
        let _ = writeln!(
            out,
            "- Start with the changed files, then shared dependencies."
        );
        return;
    }

    for cluster in graph.risky_clusters.iter().take(4) {
        let _ = writeln!(
            out,
            "- {} in `{}`: {} finding(s), max risk {}",
            cluster.rule_id, cluster.scope, cluster.count, cluster.max_score
        );
    }
    for file in graph.top_dependencies.iter().take(3) {
        let _ = writeln!(
            out,
            "- Shared dependency `{}` before downstream callers (fan-in {}, fan-out {})",
            file.path.display(),
            file.fan_in,
            file.fan_out
        );
    }
}

fn render_blast_radius(out: &mut String, graph: &ContextGraphSummary) {
    let _ = writeln!(out, "\n### Blast Radius");
    if graph.changed_blast_radius.is_empty() {
        let _ = writeln!(
            out,
            "- No import-based downstream files detected for this scope."
        );
        return;
    }

    for path in graph.changed_blast_radius.iter().take(8) {
        let _ = writeln!(out, "- `{}`", path.display());
    }
}

fn render_high_context_files(out: &mut String, graph: &ContextGraphSummary) {
    let _ = writeln!(out, "\n### High-Context Files");
    let mut rendered = 0usize;
    for file in graph.top_dependencies.iter().take(5) {
        render_file_metric(out, "dependency", file);
        rendered += 1;
    }
    for file in graph.top_hubs.iter().take(5usize.saturating_sub(rendered)) {
        render_file_metric(out, "hub", file);
    }
}

fn render_file_metric(out: &mut String, label: &str, file: &ContextGraphFileMetric) {
    let roles = if file.roles.is_empty() {
        String::new()
    } else {
        format!("; roles {}", file.roles.join(","))
    };
    let _ = writeln!(
        out,
        "- `{}`: {label}, fan-in {}, fan-out {}, instability {:.2}{}",
        file.path.display(),
        file.fan_in,
        file.fan_out,
        file.instability,
        roles
    );
}

fn render_verification_focus(out: &mut String, graph: &ContextGraphSummary) {
    let _ = writeln!(out, "\n### Verification Focus");
    if !graph.cycles.is_empty() {
        let _ = writeln!(
            out,
            "- Re-run architecture checks around {} import cycle(s).",
            graph.cycles.len()
        );
    }
    if !graph.risky_clusters.is_empty() {
        let _ = writeln!(out, "- Re-scan the top risky clusters after edits.");
    }
    if !graph.changed_blast_radius.is_empty() {
        let _ = writeln!(
            out,
            "- Test files in the blast radius, not only touched files."
        );
    }
    if graph.cycles.is_empty()
        && graph.risky_clusters.is_empty()
        && graph.changed_blast_radius.is_empty()
    {
        let _ = writeln!(
            out,
            "- Run the focused scan command for the selected severity scope."
        );
    }
}
