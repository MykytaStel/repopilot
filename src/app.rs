use crate::cli::{Cli, Commands};
use repopilot::output::render_scan_summary;
use repopilot::scan::scanner::scan_path;

pub fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Commands::Scan { path, format } => {
            let summary = scan_path(&path)?;
            let output = render_scan_summary(&summary, format.into())?;

            println!("{output}");

            Ok(())
        }
    }
}
