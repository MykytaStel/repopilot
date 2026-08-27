use crate::cli::{PriorityArg, ScanProfileArg, SeverityArg};
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
as technical debt — not as a way to silence CI.\n\n\
Pass --profile/--min-severity/--min-priority to scope the stored baseline the\n\
same way a later `scan --baseline`/`review --baseline` will be scoped. A\n\
finding hidden from one side by a filter the other side does not share reads\n\
as a false new or resolved finding, not as unchanged.",
        after_help = "EXAMPLES:\n  \
repopilot baseline create .\n  \
repopilot baseline create . --output ./baseline.json\n  \
repopilot baseline create . --config repopilot.toml\n  \
repopilot baseline create . --ignore-feedback\n  \
repopilot baseline create . --force\n  \
repopilot baseline create . --profile strict --min-severity high"
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

    /// Path to repopilot.toml config
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Do not apply local .repopilot/feedback.yml suppressions
    #[arg(long)]
    pub ignore_feedback: bool,

    /// Overwrite an existing baseline file
    #[arg(long)]
    pub force: bool,

    /// Report visibility profile the stored baseline is scoped to: default
    /// hides low-signal suggestions; strict stores all findings. Must match
    /// the profile later scans compare against, or findings hidden by one
    /// side's filter but not the other's read as spurious new/resolved.
    #[arg(long, value_enum)]
    pub profile: Option<ScanProfileArg>,

    /// Only store findings at or above this severity
    #[arg(long, value_enum)]
    pub min_severity: Option<SeverityArg>,

    /// Only store findings at or above this risk priority
    #[arg(long, value_enum)]
    pub min_priority: Option<PriorityArg>,
}
