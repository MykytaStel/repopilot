use crate::cli::CompareOutputFormatArg;
use crate::commands::product_scan::{
    ProductScanMode, ProductScanRequest, emit_report_only_diagnostics,
    enforce_diagnostics_exit_policy, run_product_scan,
};
use crate::commands::scan_config::ScanConfigOverrides;
use crate::commands::{CliExit, EXIT_USAGE};
use repopilot::findings::filter::FindingFilter;
use repopilot::findings::visibility::FindingVisibilityProfile;
use repopilot::graph::context::{ContextGraphFileMetric, ContextGraphSummary};
use repopilot::output::OutputFormat;
use repopilot::report::writer::write_report;
use repopilot::scan::types::ScanSummary;
use serde::Serialize;
use std::fmt::Write as FmtWrite;
use std::path::PathBuf;

pub fn run(
    path: PathBuf,
    config: Option<PathBuf>,
    format: CompareOutputFormatArg,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let scan_result = run_product_scan(ProductScanRequest {
        path,
        config_path: config,
        overrides: ScanConfigOverrides::default(),
        preset: None,
        mode: ProductScanMode::Full,
        no_progress: false,
        ignore_feedback: true,
        visibility_profile: FindingVisibilityProfile::Strict,
        pre_visibility_filter: FindingFilter::default(),
    })?;
    let summary = scan_result.summary;

    emit_report_only_diagnostics(&summary);
    let rendered = render_graph_inspection(&summary, OutputFormat::from(format))?;
    write_report(&rendered, output.as_deref())?;
    enforce_diagnostics_exit_policy(&summary)?;

    Ok(())
}

fn render_graph_inspection(
    summary: &ScanSummary,
    format: OutputFormat,
) -> Result<String, Box<dyn std::error::Error>> {
    match format {
        OutputFormat::Console => Ok(render_console(summary)),
        OutputFormat::Markdown => Ok(render_markdown(summary)),
        OutputFormat::Json => Ok(serde_json::to_string_pretty(
            &GraphInspectJson::from_summary(summary),
        )?),
        OutputFormat::Html | OutputFormat::Sarif => Err(Box::new(CliExit {
            code: EXIT_USAGE,
            message: format!(
                "`inspect graph` supports only console, markdown, and json output; received {}",
                output_format_name(format)
            ),
        })),
    }
}

/// Command-local diagnostics DTO for `inspect graph`.
///
/// This JSON shape is not the stable scan report contract; product report DTOs
/// live under `report::schema`.
#[derive(Serialize)]
struct GraphInspectJson<'a> {
    kind: &'static str,
    root_path: String,
    context_graph_summary: Option<&'a ContextGraphSummary>,
    context_graph_cache: Option<&'a repopilot::graph::context::ContextGraphCacheInfo>,
    diagnostics: &'a [repopilot::scan::types::ScanDiagnostic],
}

impl<'a> GraphInspectJson<'a> {
    fn from_summary(summary: &'a ScanSummary) -> Self {
        Self {
            kind: "context-graph",
            root_path: summary.root_path.to_string_lossy().to_string(),
            context_graph_summary: summary.context_graph_summary.as_ref(),
            context_graph_cache: summary.context_graph_cache.as_ref(),
            diagnostics: &summary.diagnostics,
        }
    }
}

fn render_console(summary: &ScanSummary) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "RepoPilot Context Risk Graph\n");
    let _ = writeln!(output, "Path: {}", summary.root_path.display());
    render_cache_console(&mut output, summary);

    let Some(graph) = &summary.context_graph_summary else {
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

fn render_markdown(summary: &ScanSummary) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "# RepoPilot Context Risk Graph\n");
    let _ = writeln!(output, "- **Path:** `{}`", summary.root_path.display());
    render_cache_markdown(&mut output, summary);

    let Some(graph) = &summary.context_graph_summary else {
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
    if let Some(cache) = &summary.context_graph_cache {
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
    if let Some(cache) = &summary.context_graph_cache {
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

fn output_format_name(format: OutputFormat) -> &'static str {
    match format {
        OutputFormat::Console => "console",
        OutputFormat::Html => "html",
        OutputFormat::Json => "json",
        OutputFormat::Markdown => "markdown",
        OutputFormat::Sarif => "sarif",
    }
}
