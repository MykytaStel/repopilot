use crate::cli::{
    CompareOutputFormatArg, KnowledgeSectionArg, RuleLifecycleArg, SeverityArg, SignalSourceArg,
};
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

    /// Inspect Context Risk Graph decisions for a repository
    #[command(
        about = "Inspect RepoPilot Context Risk Graph decisions",
        after_help = "EXAMPLES:\n  \
repopilot inspect graph\n  \
repopilot inspect graph . --format json\n  \
repopilot inspect graph . --format markdown --output graph.md"
    )]
    Graph(GraphInspectOptions),

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

    /// List registered rules with lifecycle and signal metadata
    #[command(
        about = "List registered RepoPilot rules",
        after_help = "EXAMPLES:\n  \
repopilot inspect rules\n  \
repopilot inspect rules --format json\n  \
repopilot inspect rules --lifecycle preview\n  \
repopilot inspect rules --source text-heuristic"
    )]
    Rules(RuleListOptions),

    /// Inspect one registered rule by rule id
    #[command(
        about = "Inspect one RepoPilot rule",
        after_help = "EXAMPLES:\n  \
repopilot inspect rule security.secret-candidate\n  \
repopilot inspect rule architecture.circular-dependency --format json"
    )]
    Rule(RuleInspectOptions),

    /// Evaluate registered rules against local fixture projects
    #[command(
        name = "eval-rules",
        about = "Evaluate rules against local fixture projects",
        after_help = "EXAMPLES:\n  \
repopilot inspect eval-rules\n  \
repopilot inspect eval-rules --rule security.secret-candidate\n  \
repopilot inspect eval-rules --format json"
    )]
    EvalRules(RuleEvalOptions),
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
pub struct GraphInspectOptions {
    /// Repository or project path whose Context Risk Graph should be inspected
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Path to a repopilot.toml config file
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Output format for graph diagnostics
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

#[derive(Args)]
pub struct RuleListOptions {
    /// Output format for rule catalog
    #[arg(long, value_enum, default_value = "console")]
    pub format: CompareOutputFormatArg,

    /// Filter by lifecycle
    #[arg(long, value_enum)]
    pub lifecycle: Option<RuleLifecycleArg>,

    /// Filter by signal source
    #[arg(long, value_enum)]
    pub source: Option<SignalSourceArg>,

    /// Write report to a file instead of stdout
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Args)]
pub struct RuleInspectOptions {
    /// Rule ID to inspect
    pub rule_id: String,

    /// Output format for rule details
    #[arg(long, value_enum, default_value = "console")]
    pub format: CompareOutputFormatArg,

    /// Write report to a file instead of stdout
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Args)]
pub struct RuleEvalOptions {
    /// Evaluate only one rule ID
    #[arg(long)]
    pub rule: Option<String>,

    /// Fixture root, defaults to tests/fixtures/rules
    #[arg(long)]
    pub fixtures: Option<PathBuf>,

    /// Output format for evaluation report
    #[arg(long, value_enum, default_value = "console")]
    pub format: CompareOutputFormatArg,

    /// Write report to a file instead of stdout
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}
