use crate::cli::{CompareOutputFormatArg, KnowledgeSectionArg, SeverityArg};
use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Args)]
pub struct InspectOptions {
    #[command(subcommand)]
    pub command: InspectCommands,
}

#[derive(Subcommand)]
pub enum InspectCommands {
    /// Explain file context classification and optional rule decisions
    #[command(
        about = "Explain file context classification and rule decisions",
        after_help = "EXAMPLES:\n  \
repopilot inspect explain src/main.rs\n  \
repopilot inspect explain src/main.rs --rule language.rust.panic-risk --signal rust.unwrap\n  \
repopilot inspect explain src/App.tsx --format markdown --output explain.md"
    )]
    Explain(ExplainOptions),

    /// Inspect bundled language, framework, runtime, paradigm, and rule knowledge
    #[command(
        about = "Inspect RepoPilot bundled knowledge",
        after_help = "EXAMPLES:\n  \
repopilot inspect knowledge\n  \
repopilot inspect knowledge --section languages\n  \
repopilot inspect knowledge --section rules --format json"
    )]
    Knowledge(KnowledgeOptions),

    /// Inspect local changed-scan cache diagnostics
    #[command(
        about = "Inspect RepoPilot local cache diagnostics",
        after_help = "EXAMPLES:\n  \
repopilot inspect cache\n  \
repopilot inspect cache . --format json\n  \
repopilot inspect cache . --format markdown --output cache.md"
    )]
    Cache(CacheInspectOptions),

    /// Inspect local feedback suppressions and validation diagnostics
    #[command(
        about = "Inspect RepoPilot local feedback suppressions",
        after_help = "EXAMPLES:\n  \
repopilot inspect feedback\n  \
repopilot inspect feedback . --format json\n  \
repopilot inspect feedback . --evaluate --format json\n  \
repopilot inspect feedback . --format markdown --output feedback.md"
    )]
    Feedback(FeedbackInspectOptions),
}

#[derive(Args)]
pub struct ExplainOptions {
    /// File path to classify
    pub path: PathBuf,

    /// Rule ID to evaluate against the file context
    #[arg(long)]
    pub rule: Option<String>,

    /// Optional rule signal, for example rust.unwrap or rust.panic
    #[arg(long)]
    pub signal: Option<String>,

    /// Base severity used before Knowledge Engine overrides
    #[arg(long, value_enum, default_value = "medium")]
    pub severity: SeverityArg,

    /// Output format for the explanation
    #[arg(long, value_enum, default_value = "console")]
    pub format: CompareOutputFormatArg,

    /// Write report to a file instead of stdout
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Args)]
pub struct KnowledgeOptions {
    /// Catalog section to render
    #[arg(long, value_enum, default_value = "all")]
    pub section: KnowledgeSectionArg,

    /// Output format for the catalog
    #[arg(long, value_enum, default_value = "console")]
    pub format: CompareOutputFormatArg,

    /// Write report to a file instead of stdout
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Args)]
pub struct CacheInspectOptions {
    /// Repository or project path whose .repopilot/cache directory should be inspected
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Output format for cache diagnostics
    #[arg(long, value_enum, default_value = "console")]
    pub format: CompareOutputFormatArg,

    /// Write report to a file instead of stdout
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Args)]
pub struct FeedbackInspectOptions {
    /// Repository or project path whose .repopilot/feedback.yml should be inspected
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Output format for feedback diagnostics
    #[arg(long, value_enum, default_value = "console")]
    pub format: CompareOutputFormatArg,

    /// Run a repository scan and evaluate suppressions against current findings
    #[arg(long)]
    pub evaluate: bool,

    /// Write report to a file instead of stdout
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}
