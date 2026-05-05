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
    /// Compare two JSON scan reports and show what changed
    Compare {
        /// Path to the earlier scan report (JSON)
        before: std::path::PathBuf,

        /// Path to the more recent scan report (JSON)
        after: std::path::PathBuf,

        /// Output format
        #[arg(long, value_enum, default_value = "console")]
        format: OutputFormatArg,

        /// Write report to a file instead of stdout
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
    },

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

        /// Maximum non-empty LOC before a file is reported as large (default: 300)
        #[arg(long)]
        max_file_loc: Option<usize>,

        /// Maximum number of files in a single directory before flagging (default: 20)
        #[arg(long)]
        max_directory_modules: Option<usize>,

        /// Maximum directory nesting depth before flagging (default: 5)
        #[arg(long)]
        max_directory_depth: Option<usize>,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum OutputFormatArg {
    Console,
    Html,
    Json,
    Markdown,
}

impl From<OutputFormatArg> for OutputFormat {
    fn from(format: OutputFormatArg) -> Self {
        match format {
            OutputFormatArg::Console => OutputFormat::Console,
            OutputFormatArg::Html => OutputFormat::Html,
            OutputFormatArg::Json => OutputFormat::Json,
            OutputFormatArg::Markdown => OutputFormat::Markdown,
        }
    }
}
