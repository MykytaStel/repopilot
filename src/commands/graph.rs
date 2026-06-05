mod render;

use crate::cli::GraphOutputFormatArg;
use crate::commands::product_scan::{
    ProductScanMode, ProductScanRequest, emit_report_only_diagnostics,
    enforce_diagnostics_exit_policy, run_product_scan,
};
use crate::commands::scan_config::ScanConfigOverrides;

use repopilot::findings::filter::FindingFilter;
use repopilot::findings::visibility::FindingVisibilityProfile;
use repopilot::graph::context::ContextGraphSummary;
use repopilot::report::writer::write_report;
use repopilot::scan::types::ScanSummary;
use serde::Serialize;
use std::path::PathBuf;

pub fn run(
    path: PathBuf,
    config: Option<PathBuf>,
    format: GraphOutputFormatArg,
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
    let rendered = render_graph_inspection(&summary, format)?;
    write_report(&rendered, output.as_deref())?;
    enforce_diagnostics_exit_policy(&summary)?;

    Ok(())
}

fn render_graph_inspection(
    summary: &ScanSummary,
    format: GraphOutputFormatArg,
) -> Result<String, Box<dyn std::error::Error>> {
    match format {
        GraphOutputFormatArg::Console => Ok(render::render_console(summary)),
        GraphOutputFormatArg::Markdown => Ok(render::render_markdown(summary)),
        GraphOutputFormatArg::Json => Ok(serde_json::to_string_pretty(
            &GraphInspectJson::from_summary(summary),
        )?),
        GraphOutputFormatArg::Dot => Ok(render::render_dot(summary)),
        GraphOutputFormatArg::Mermaid => Ok(render::render_mermaid(summary)),
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
            context_graph_summary: summary.artifacts.context_graph_summary.as_ref(),
            context_graph_cache: summary.artifacts.context_graph_cache.as_ref(),
            diagnostics: &summary.artifacts.diagnostics,
        }
    }
}
