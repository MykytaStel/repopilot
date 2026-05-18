use crate::cli::CompareOutputFormatArg;
use repopilot::output::OutputFormat;
use repopilot::report::writer::write_report;
use repopilot::scan::cache::{CacheDiagnostics, inspect_cache};
use std::path::PathBuf;

pub fn run(
    path: PathBuf,
    format: CompareOutputFormatArg,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let diagnostics = inspect_cache(&path);
    let rendered = render_cache_diagnostics(&diagnostics, OutputFormat::from(format))?;
    write_report(&rendered, output.as_deref())?;
    Ok(())
}

fn render_cache_diagnostics(
    diagnostics: &CacheDiagnostics,
    format: OutputFormat,
) -> Result<String, serde_json::Error> {
    match format {
        OutputFormat::Console => Ok(render_console(diagnostics)),
        OutputFormat::Markdown => Ok(render_markdown(diagnostics)),
        OutputFormat::Json | OutputFormat::Html | OutputFormat::Sarif => {
            serde_json::to_string_pretty(diagnostics)
        }
    }
}

fn render_console(diagnostics: &CacheDiagnostics) -> String {
    format!(
        "RepoPilot Cache Diagnostics\n\nCache dir: {}\nExists: {}\nSchema version: {}\nRepoPilot version: {}\nApprox size: {} bytes\nFile hashes: {}\nFile roles: {}\nFindings entries: {}\nStale entries: {}\n",
        diagnostics.cache_dir.display(),
        yes_no(diagnostics.exists),
        diagnostics.schema_version,
        diagnostics.repopilot_version,
        diagnostics.approximate_size_bytes,
        diagnostics.file_hashes_count,
        diagnostics.file_roles_count,
        diagnostics.findings_count,
        diagnostics.stale_entries_count,
    )
}

fn render_markdown(diagnostics: &CacheDiagnostics) -> String {
    format!(
        "# RepoPilot Cache Diagnostics\n\n- **Cache dir:** `{}`\n- **Exists:** {}\n- **Schema version:** `{}`\n- **RepoPilot version:** `{}`\n- **Approx size:** {} bytes\n- **File hashes:** {}\n- **File roles:** {}\n- **Findings entries:** {}\n- **Stale entries:** {}\n",
        diagnostics.cache_dir.display(),
        yes_no(diagnostics.exists),
        diagnostics.schema_version,
        diagnostics.repopilot_version,
        diagnostics.approximate_size_bytes,
        diagnostics.file_hashes_count,
        diagnostics.file_roles_count,
        diagnostics.findings_count,
        diagnostics.stale_entries_count,
    )
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}
