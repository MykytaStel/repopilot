use crate::cli::{CompareOutputFormatArg, FailOnArg, PriorityArg, SeverityArg};
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct ReviewOptions {
    /// Path to project, folder, or file to review
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Base Git ref for branch/CI review; defaults to working tree vs HEAD
    #[arg(long)]
    pub base: Option<String>,

    /// Head Git ref for branch/CI review; requires --base
    #[arg(long)]
    pub head: Option<String>,

    /// Path to a repopilot.toml config file
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Path to a baseline file for new/existing finding status
    #[arg(long)]
    pub baseline: Option<PathBuf>,

    /// Exit with code 1 when in-diff findings meet this severity threshold
    #[arg(long, value_enum)]
    pub fail_on: Option<FailOnArg>,

    /// Exit with code 1 when in-diff findings meet this risk priority threshold
    #[arg(long, value_enum)]
    pub fail_on_priority: Option<PriorityArg>,

    /// Output format for the review report
    #[arg(long, value_enum, default_value = "console")]
    pub format: CompareOutputFormatArg,

    /// Write report to a file instead of stdout
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Override the large-file LOC threshold
    #[arg(long)]
    pub max_file_loc: Option<usize>,

    /// Override the maximum files per directory before architecture findings
    #[arg(long)]
    pub max_directory_modules: Option<usize>,

    /// Override the maximum directory nesting depth before architecture findings
    #[arg(long)]
    pub max_directory_depth: Option<usize>,

    /// Only render findings at or above this severity
    #[arg(long, value_enum)]
    pub min_severity: Option<SeverityArg>,

    /// Only render findings at or above this risk priority
    #[arg(long, value_enum)]
    pub min_priority: Option<PriorityArg>,
}
