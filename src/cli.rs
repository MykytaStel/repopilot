use clap::{Parser, Subcommand, ValueEnum};
use repopilot::output::OutputFormat;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "repopilot")]
#[command(about = "Local-first codebase audit CLI", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Scan a project, folder, or file
    Scan {
        /// Path to project, folder, or file
        path: PathBuf,

        /// Output format
        #[arg(long, value_enum, default_value = "console")]
        format: OutputFormatArg,

        /// Write report to a file instead of stdout
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Maximum non-empty LOC before a file is reported as large
        #[arg(long)]
        max_file_loc: Option<usize>,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum OutputFormatArg {
    Console,
    Json,
    Markdown,
}

impl From<OutputFormatArg> for OutputFormat {
    fn from(format: OutputFormatArg) -> Self {
        match format {
            OutputFormatArg::Console => OutputFormat::Console,
            OutputFormatArg::Json => OutputFormat::Json,
            OutputFormatArg::Markdown => OutputFormat::Markdown,
        }
    }
}
