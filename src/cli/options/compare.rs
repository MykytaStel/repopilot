use crate::cli::CompareOutputFormatArg;
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct CompareOptions {
    /// Earlier RepoPilot JSON scan report
    pub before: PathBuf,

    /// Later RepoPilot JSON scan report
    pub after: PathBuf,

    /// Output format for the comparison report
    #[arg(long, value_enum, default_value = "console")]
    pub format: CompareOutputFormatArg,

    /// Write report to a file instead of stdout
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}
