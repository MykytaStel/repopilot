use crate::cli::{Cli, Commands};
use repopilot::output::render_scan_summary;
use repopilot::report::writer::write_report;
use repopilot::scan::scanner::scan_path;

pub fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Commands::Scan {
            path,
            format,
            output,
        } => {
            let summary = scan_path(&path)?;
            let rendered_report = render_scan_summary(&summary, format.into())?;

            write_report(&rendered_report, output.as_deref())?;

            Ok(())
        }
    }
}
