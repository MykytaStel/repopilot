use crate::cli::CompareOutputFormatArg;
use crate::commands::{ScanConfigOverrides, build_scan_config};
use repopilot::config::loader::{load_default_config, load_optional_config};
use repopilot::config::model::RepoPilotConfig;
use repopilot::doctor::{build_doctor_report, render_doctor_report};
use repopilot::output::OutputFormat;
use repopilot::report::writer::write_report;
use std::path::PathBuf;

pub fn run(
    path: PathBuf,
    config: Option<PathBuf>,
    format: CompareOutputFormatArg,
    output: Option<PathBuf>,
    include_low_signal: bool,
    max_files: Option<usize>,
) -> Result<(), Box<dyn std::error::Error>> {
    let repo_config = load_doctor_config(config.as_ref());

    let scan_config = build_scan_config(
        &repo_config,
        ScanConfigOverrides {
            include_low_signal,
            max_files,
            ..ScanConfigOverrides::default()
        },
    );

    let output_format: OutputFormat = format.into();

    let report = build_doctor_report(&path, config.as_deref(), &scan_config)?;
    let rendered = render_doctor_report(&report, output_format)?;

    write_report(&rendered, output.as_deref())?;

    Ok(())
}

fn load_doctor_config(config: Option<&PathBuf>) -> RepoPilotConfig {
    let result = match config {
        Some(config_path) => load_optional_config(config_path),
        None => load_default_config(),
    };

    result.unwrap_or_default()
}
