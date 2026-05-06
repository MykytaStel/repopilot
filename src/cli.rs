use clap::{Parser, Subcommand, ValueEnum};
use repopilot::baseline::gate::FailOn;
use repopilot::findings::types::Severity;
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
    /// Manage accepted baseline findings
    Baseline {
        #[command(subcommand)]
        command: BaselineCommands,
    },

    /// Compare two JSON scan reports and show what changed
    Compare {
        /// Path to the earlier scan report (JSON)
        before: std::path::PathBuf,

        /// Path to the more recent scan report (JSON)
        after: std::path::PathBuf,

        /// Output format
        #[arg(long, value_enum, default_value = "console")]
        format: CompareOutputFormatArg,

        /// Write report to a file instead of stdout
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
    },

    /// Scan a project, folder, or file
    Scan {
        /// Path to project, folder, or file
        path: PathBuf,

        /// Output format
        #[arg(long, value_enum)]
        format: Option<OutputFormatArg>,

        /// Write report to a file instead of stdout
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Path to a RepoPilot config file
        #[arg(long)]
        config: Option<PathBuf>,

        /// Path to a RepoPilot baseline file
        #[arg(long)]
        baseline: Option<PathBuf>,

        /// Fail with exit code 1 when findings meet the selected threshold
        #[arg(long, value_enum)]
        fail_on: Option<FailOnArg>,

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

    /// Generate a default repopilot.toml configuration file
    Init {
        /// Overwrite an existing config file
        #[arg(long)]
        force: bool,

        /// Path where the config file should be written
        #[arg(long, default_value = "repopilot.toml")]
        path: PathBuf,
    },
}

#[derive(Subcommand)]
pub enum BaselineCommands {
    /// Scan a path and store the current findings as accepted debt
    Create {
        /// Path to project, folder, or file
        path: PathBuf,

        /// Write baseline to a custom path
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Overwrite an existing baseline file
        #[arg(long)]
        force: bool,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum OutputFormatArg {
    Console,
    Html,
    Json,
    Markdown,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum CompareOutputFormatArg {
    Console,
    Json,
    Markdown,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum FailOnArg {
    NewLow,
    NewMedium,
    NewHigh,
    NewCritical,
    Low,
    Medium,
    High,
    Critical,
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

impl From<FailOnArg> for FailOn {
    fn from(value: FailOnArg) -> Self {
        match value {
            FailOnArg::NewLow => FailOn::New(Severity::Low),
            FailOnArg::NewMedium => FailOn::New(Severity::Medium),
            FailOnArg::NewHigh => FailOn::New(Severity::High),
            FailOnArg::NewCritical => FailOn::New(Severity::Critical),
            FailOnArg::Low => FailOn::Any(Severity::Low),
            FailOnArg::Medium => FailOn::Any(Severity::Medium),
            FailOnArg::High => FailOn::Any(Severity::High),
            FailOnArg::Critical => FailOn::Any(Severity::Critical),
        }
    }
}
