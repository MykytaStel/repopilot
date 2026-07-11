use crate::cli::{
    CompareOutputFormatArg, ConfidenceArg, FailOnArg, MaxFindingsArg, PriorityArg, ReviewDetailArg,
    ReviewFailOnArg, ReviewScopeArg, ScanProfileArg, SeverityArg, parse_max_findings,
};
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

    /// Review everything since the last `repopilot snapshot` (commits and
    /// uncommitted edits); cannot be combined with --base/--head
    #[arg(long, conflicts_with_all = ["base", "head"])]
    pub since_snapshot: bool,

    /// Review only changed files (default) or include full-repository findings
    #[arg(long, value_enum)]
    pub scope: Option<ReviewScopeArg>,

    /// Finding visibility profile; defaults to default for changed scope and strict for full scope
    #[arg(long, value_enum)]
    pub profile: Option<ScanProfileArg>,

    /// Path to a repopilot.toml config file
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Path to a baseline file for new/existing finding status
    #[arg(long)]
    pub baseline: Option<PathBuf>,

    /// Finding gate: exit 1 when in-diff findings meet this severity threshold (excludes --fail-on-priority)
    #[arg(long, value_enum)]
    pub fail_on: Option<FailOnArg>,

    /// Finding gate: exit 1 when in-diff findings meet this risk-priority threshold (excludes --fail-on)
    #[arg(long, value_enum)]
    pub fail_on_priority: Option<PriorityArg>,

    /// Review-signal gate: exit 1 on gate-eligible definitely-sensitive signals; config peer [review] fail_on
    #[arg(long, value_enum)]
    pub fail_on_review: Option<ReviewFailOnArg>,

    /// Output format for the review report
    #[arg(long, value_enum, default_value = "console")]
    pub format: CompareOutputFormatArg,

    /// Progressive console disclosure: verdict only, top findings, or full evidence
    #[arg(long, value_enum)]
    pub detail: Option<ReviewDetailArg>,

    /// Limit rendered console findings; use 'none' for all
    #[arg(long, value_name = "N|none", value_parser = parse_max_findings)]
    pub max_findings: Option<MaxFindingsArg>,

    /// Disable progress indicators
    #[arg(long)]
    pub no_progress: bool,

    /// Write report to a file instead of stdout
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Write an additional SARIF report without running the review twice
    #[arg(long, value_name = "PATH")]
    pub sarif_output: Option<PathBuf>,

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

    /// Only render findings at or above this confidence
    #[arg(long, value_enum)]
    pub min_confidence: Option<ConfidenceArg>,

    /// Only render findings at or above this risk priority
    #[arg(long, value_enum)]
    pub min_priority: Option<PriorityArg>,

    /// Ignore .repopilot/feedback.yml local suppressions
    #[arg(long)]
    pub ignore_feedback: bool,
}
