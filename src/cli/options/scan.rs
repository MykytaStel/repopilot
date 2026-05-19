use crate::cli::{
    ConfidenceArg, FailOnArg, OutputFormatArg, PriorityArg, ScanProfileArg, SeverityArg,
    parse_byte_size,
};
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct ScanOptions {
    /// Path to project, folder, or file to scan
    pub path: PathBuf,

    /// Output format; defaults to repopilot.toml output.default_format or console
    #[arg(long, value_enum)]
    pub format: Option<OutputFormatArg>,

    /// Write report to a file instead of stdout
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Write a reproducible audit receipt JSON file
    #[arg(long, value_name = "PATH")]
    pub receipt: Option<PathBuf>,

    /// Path to a repopilot.toml config file
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Path to a baseline file for new/existing finding status
    #[arg(long)]
    pub baseline: Option<PathBuf>,

    /// Exit with code 1 when findings meet this severity/status threshold
    #[arg(long, value_enum)]
    pub fail_on: Option<FailOnArg>,

    /// Exit with code 1 when findings meet this risk priority threshold
    #[arg(long, value_enum)]
    pub fail_on_priority: Option<PriorityArg>,

    /// Override the large-file LOC threshold
    #[arg(long)]
    pub max_file_loc: Option<usize>,

    /// Override the maximum files per directory before architecture findings
    #[arg(long)]
    pub max_directory_modules: Option<usize>,

    /// Override the maximum directory nesting depth before architecture findings
    #[arg(long)]
    pub max_directory_depth: Option<usize>,

    /// Exclude a path or file/directory name after gitignore processing; repeatable
    #[arg(long, value_name = "PATH_OR_NAME")]
    pub exclude: Vec<String>,

    /// Analyze test, fixture, example, generated, and benchmark paths skipped by default
    #[arg(long)]
    pub include_low_signal: bool,

    /// Skip files larger than SIZE (bytes, kb, mb, or gb; 0 disables the guard)
    #[arg(long, value_name = "SIZE", value_parser = parse_byte_size)]
    pub max_file_size: Option<u64>,

    /// Analyze at most N discovered files after ignore and exclude filters
    #[arg(long, value_name = "N")]
    pub max_files: Option<usize>,

    /// Scan each detected workspace package separately and group findings by package
    #[arg(long, short = 'w')]
    pub workspace: bool,

    /// Scan only files changed against HEAD, including untracked files
    #[arg(long)]
    pub changed: bool,

    /// Scan only files changed since BASE ref, compared with HEAD
    #[arg(long, value_name = "BASE")]
    pub since: Option<String>,

    /// Only render findings at or above this severity
    #[arg(long, value_enum)]
    pub min_severity: Option<SeverityArg>,

    /// Only render findings at or above this confidence
    #[arg(long, value_enum)]
    pub min_confidence: Option<ConfidenceArg>,

    /// Only render findings at or above this risk priority
    #[arg(long, value_enum)]
    pub min_priority: Option<PriorityArg>,

    /// Print scan and render timing to stderr
    #[arg(long)]
    pub verbose: bool,

    /// Report visibility profile: default hides low-signal suggestions; strict shows all findings
    #[arg(long, value_enum)]
    pub profile: Option<ScanProfileArg>,

    /// Include maintainability and testing suggestions hidden by the default profile
    #[arg(long)]
    pub include_maintainability: bool,

    /// Print per-phase scan timing breakdown to stderr
    #[arg(long)]
    pub timing: bool,

    /// Apply threshold presets without editing repopilot.toml
    #[arg(long, value_parser = ["strict", "balanced", "lenient"])]
    pub preset: Option<String>,

    /// Ignore .repopilot/feedback.yml local suppressions
    #[arg(long)]
    pub ignore_feedback: bool,

    /// Only show findings for specific rule ID(s); repeatable
    #[arg(long, value_name = "RULE_ID")]
    pub rule: Vec<String>,
}
