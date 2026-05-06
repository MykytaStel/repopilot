use crate::cli::{FailOnArg, OutputFormatArg};
use crate::commands::{CliExit, build_scan_config};
use repopilot::baseline::diff::{all_findings_new, diff_summary_against_baseline};
use repopilot::baseline::gate::evaluate_ci_gate;
use repopilot::baseline::reader::read_baseline;
use repopilot::config::loader::{load_default_config, load_optional_config};
use repopilot::output::{render_baseline_scan_report, render_scan_summary};
use repopilot::report::writer::write_report;
use repopilot::scan::scanner::scan_path_with_config;
use std::path::PathBuf;

#[allow(clippy::too_many_arguments)]
pub fn run(
    path: PathBuf,
    format: Option<OutputFormatArg>,
    output: Option<PathBuf>,
    config: Option<PathBuf>,
    baseline: Option<PathBuf>,
    fail_on: Option<FailOnArg>,
    max_file_loc: Option<usize>,
    max_directory_modules: Option<usize>,
    max_directory_depth: Option<usize>,
) -> Result<(), Box<dyn std::error::Error>> {
    let repo_config = match config {
        Some(config_path) => load_optional_config(&config_path)?,
        None => load_default_config()?,
    };
    let scan_config = build_scan_config(&repo_config, max_file_loc, max_directory_modules, max_directory_depth);
    let output_format = format
        .map(Into::into)
        .unwrap_or(repo_config.output.default_format);
    let summary = scan_path_with_config(&path, &scan_config)?;

    if baseline.is_some() || fail_on.is_some() {
        let baseline_report = match baseline {
            Some(baseline_path) => {
                let baseline_file = read_baseline(&baseline_path)?;
                diff_summary_against_baseline(summary, &baseline_file, baseline_path)
            }
            None => all_findings_new(summary),
        };

        let ci_gate = fail_on
            .map(Into::into)
            .map(|fail_on| evaluate_ci_gate(&baseline_report, fail_on));
        let rendered_report =
            render_baseline_scan_report(&baseline_report, output_format, ci_gate.as_ref())?;

        write_report(&rendered_report, output.as_deref())?;

        if let Some(ci_gate) = ci_gate
            && let Some(message) = ci_gate.failure_message()
        {
            return Err(Box::new(CliExit { code: 1, message }));
        }

        return Ok(());
    }

    let rendered_report = render_scan_summary(&summary, output_format)?;
    write_report(&rendered_report, output.as_deref())?;

    Ok(())
}
