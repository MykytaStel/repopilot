use crate::cli::CompareOutputFormatArg;
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct InitOptions {
    /// Overwrite an existing config file
    #[arg(long)]
    pub force: bool,

    /// Config file path to write
    #[arg(long, default_value = "repopilot.toml")]
    pub path: PathBuf,
}

#[derive(Args)]
pub struct DoctorOptions {
    /// Path to diagnose
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Path to a repopilot.toml config file
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Output format for diagnostics
    #[arg(long, value_enum, default_value = "console")]
    pub format: CompareOutputFormatArg,

    /// Write report to a file instead of stdout
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Analyze test, fixture, example, generated, and benchmark paths skipped by default
    #[arg(long)]
    pub include_low_signal: bool,

    /// Analyze at most N discovered files after ignore and exclude filters
    #[arg(long, value_name = "N")]
    pub max_files: Option<usize>,
}
