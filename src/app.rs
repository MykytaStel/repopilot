use crate::cli::{Cli, Commands};
use repopilot::output::render_scan_summary;
use repopilot::report::writer::write_report;
use repopilot::scan::config::ScanConfig;
use repopilot::scan::scanner::scan_path_with_config;

pub fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Commands::Scan {
            path,
            format,
            output,
            max_file_loc,
        } => {
            let config = build_scan_config(max_file_loc);
            let summary = scan_path_with_config(&path, &config)?;
            let rendered_report = render_scan_summary(&summary, format.into())?;

            write_report(&rendered_report, output.as_deref())?;

            Ok(())
        }
    }
}

fn build_scan_config(max_file_loc: Option<usize>) -> ScanConfig {
    match max_file_loc {
        Some(threshold) => ScanConfig::default().with_large_file_loc_threshold(threshold),
        None => ScanConfig::default(),
    }
}
