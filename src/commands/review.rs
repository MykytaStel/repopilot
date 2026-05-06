use crate::cli::{CompareOutputFormatArg, FailOnArg};
use crate::commands::{CliExit, build_scan_config};
use repopilot::baseline::gate::evaluate_ci_gate;
use repopilot::baseline::reader::read_baseline;
use repopilot::config::loader::{load_default_config, load_optional_config};
use repopilot::report::writer::write_report;
use repopilot::review::render::render;
use repopilot::review::{build_review_report, review_report_for_ci};
use repopilot::scan::scanner::scan_path_with_config;
use std::path::PathBuf;

#[allow(clippy::too_many_arguments)]
pub fn run(
    path: PathBuf,
    base: Option<String>,
    head: Option<String>,
    config: Option<PathBuf>,
    baseline: Option<PathBuf>,
    fail_on: Option<FailOnArg>,
    format: CompareOutputFormatArg,
    output: Option<PathBuf>,
    max_file_loc: Option<usize>,
    max_directory_modules: Option<usize>,
    max_directory_depth: Option<usize>,
) -> Result<(), Box<dyn std::error::Error>> {
    if base.is_none() && head.is_some() {
        return Err(Box::new(CliExit {
            code: 1,
            message: "`repopilot review --head` requires --base".to_string(),
        }));
    }

    let repo_config = match config {
        Some(config_path) => load_optional_config(&config_path)?,
        None => load_default_config()?,
    };
    let scan_config = build_scan_config(&repo_config, max_file_loc, max_directory_modules, max_directory_depth);
    let summary = scan_path_with_config(&path, &scan_config)?;
    let baseline_file = match baseline {
        Some(baseline_path) => Some((read_baseline(&baseline_path)?, baseline_path)),
        None => None,
    };
    let baseline_ref = baseline_file
        .as_ref()
        .map(|(baseline, path)| (baseline, path.clone()));
    let review_report = build_review_report(
        summary,
        &path,
        base.as_deref(),
        head.as_deref(),
        baseline_ref,
    )?;
    let ci_report = review_report_for_ci(&review_report);
    let ci_gate = fail_on
        .map(Into::into)
        .map(|fail_on| evaluate_ci_gate(&ci_report, fail_on));
    let rendered_report = render(&review_report, format.into(), ci_gate.as_ref())?;

    write_report(&rendered_report, output.as_deref())?;

    if let Some(ci_gate) = ci_gate
        && let Some(message) = ci_gate.failure_message()
    {
        return Err(Box::new(CliExit { code: 1, message }));
    }

    Ok(())
}
