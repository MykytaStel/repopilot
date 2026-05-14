use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Args)]
pub struct BaselineOptions {
    #[command(subcommand)]
    pub command: BaselineCommands,
}

#[derive(Subcommand)]
pub enum BaselineCommands {
    /// Scan a path and store the current findings as accepted debt
    #[command(
        about = "Scan a path and store the current findings as accepted debt",
        long_about = "Runs a full scan and writes all current findings to a baseline file.\n\n\
Future scans with `--baseline` will mark each matching finding as `existing` and\n\
flag only genuinely new findings. This lets CI gate on `--fail-on new-high` without\n\
failing on pre-existing issues.\n\n\
By default writes to .repopilot/baseline.json and creates the directory if needed.\n\
Existing baseline files are not overwritten unless you pass `--force`.\n\n\
Refresh the baseline only when the team explicitly accepts the current findings\n\
as technical debt — not as a way to silence CI.",
        after_help = "EXAMPLES:\n  \
repopilot baseline create .\n  \
repopilot baseline create . --output ./baseline.json\n  \
repopilot baseline create . --force"
    )]
    Create(BaselineCreateOptions),
}

#[derive(Args)]
pub struct BaselineCreateOptions {
    /// Path to scan before writing the baseline
    pub path: PathBuf,

    /// Baseline output path; defaults to .repopilot/baseline.json
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Overwrite an existing baseline file
    #[arg(long)]
    pub force: bool,
}
