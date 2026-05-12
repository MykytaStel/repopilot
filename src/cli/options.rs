use crate::cli::{
    CompareOutputFormatArg, FailOnArg, KnowledgeSectionArg, OutputFormatArg, SeverityArg,
    parse_byte_size, parse_vibe_budget,
};
use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum Commands {
    /// Manage accepted baseline findings (alias: bl)
    #[command(
        alias = "bl",
        about = "Manage accepted baseline findings",
        long_about = "A baseline stores findings that are accepted as existing technical debt.\n\n\
Future scans can mark each finding as `new` or `existing`, which is useful for\n\
adopting RepoPilot in a legacy repository without failing CI on pre-existing issues.\n\n\
Run `repopilot baseline create` to snapshot the current findings, then pass\n\
`--baseline` to `scan` or `review` to suppress accepted findings in reports.",
        after_help = "EXAMPLES:\n  \
repopilot baseline create .\n  \
repopilot baseline create . --output ./baseline.json\n  \
repopilot baseline create . --force   # overwrite existing baseline"
    )]
    Baseline(BaselineOptions),

    /// Compare two JSON scan reports and show what changed (alias: cmp)
    #[command(
        alias = "cmp",
        about = "Compare two JSON scan reports and show what changed",
        long_about = "Diffs two RepoPilot JSON scan reports and reports which findings are new,\n\
resolved, or unchanged between them.\n\n\
Typical workflow:\n  \
1. Scan the project before a change: `repopilot scan . --format json --output before.json`\n  \
2. Make your changes.\n  \
3. Scan again: `repopilot scan . --format json --output after.json`\n  \
4. Diff the two: `repopilot compare before.json after.json`\n\n\
Output can be formatted as console (default), JSON, or Markdown.",
        after_help = "EXAMPLES:\n  \
repopilot compare before.json after.json\n  \
repopilot compare before.json after.json --format markdown\n  \
repopilot compare before.json after.json --format json --output diff.json"
    )]
    Compare(CompareOptions),

    /// Scan a project, folder, or file for findings (alias: s)
    #[command(
        alias = "s",
        about = "Scan a project, folder, or file for findings",
        long_about = "Walks the target path and runs all enabled audit rules:\n\n\
  Architecture  — oversized files, deep nesting, too many modules per directory\n  \
Coupling      — excessive fan-out, high-instability hubs, circular dependencies\n  \
Code quality  — cyclomatic complexity, long functions, TODO/FIXME/HACK markers\n  \
Security      — hardcoded secret candidates, committed private keys, .env files\n  \
Testing       — missing test folder, source files without test counterparts\n\n\
The scan respects .gitignore, .repopilotignore, built-in ignores, and --exclude.\n\
Low-signal test, fixture, example, generated, and benchmark paths are skipped by\n\
default unless --include-low-signal is passed.\n\n\
Use `--baseline` to mark findings as new or existing. Use `--fail-on` to set a\n\
CI failure threshold. Use `--format sarif` to upload results to GitHub Code Scanning.",
        after_help = "EXAMPLES:\n  \
repopilot scan .\n  \
repopilot scan src/\n  \
repopilot scan . --format json --output report.json\n  \
repopilot scan . --format sarif --output repopilot.sarif\n  \
repopilot scan . --format html --output report.html\n  \
repopilot scan . --config repopilot.toml\n  \
repopilot scan . --baseline .repopilot/baseline.json\n  \
repopilot scan . --baseline .repopilot/baseline.json --fail-on new-high\n  \
repopilot scan . --max-file-loc 500 --max-directory-modules 30\n  \
repopilot scan . --exclude generated --max-file-size 1mb --max-files 1000"
    )]
    Scan(ScanOptions),

    /// Review findings that touch changed Git diff lines (alias: r)
    #[command(
        alias = "r",
        about = "Review findings that touch changed Git diff lines",
        long_about = "Scans the repository and separates findings into two groups:\n  \
in-diff   — findings on lines that appear in the current Git diff\n  \
out-of-diff — findings elsewhere in the codebase\n\n\
By default, review compares the working tree against HEAD, covering staged, unstaged,\n\
and untracked changes. For branch or CI review, pass a base ref with `--base`.\n\n\
When coupling data is available, review also shows blast radius: files that import\n\
changed files and may need extra attention.\n\n\
When `--fail-on` is used, the CI gate evaluates only in-diff findings so unrelated\n\
pre-existing issues do not block the pipeline.",
        after_help = "EXAMPLES:\n  \
repopilot review .\n  \
repopilot review . --base origin/main\n  \
repopilot review . --base origin/main --head HEAD\n  \
repopilot review . --base origin/main --format markdown --output review.md\n  \
repopilot review . --baseline .repopilot/baseline.json --fail-on new-high\n  \
repopilot review . --format json --output review.json"
    )]
    Review(ReviewOptions),

    /// Generate local AI-ready context, plans, and prompts
    #[command(
        about = "Generate local AI-ready context, plans, and prompts",
        long_about = "Groups RepoPilot's AI-assisted remediation outputs under one stable command family.\n\n\
These commands scan locally and emit Markdown. RepoPilot does not call AI services\n\
or upload source code.",
        after_help = "EXAMPLES:\n  \
repopilot ai context . --focus security\n  \
repopilot ai plan . --budget 4k\n  \
repopilot ai prompt . --output prompt.md"
    )]
    Ai(AiOptions),

    /// Inspect RepoPilot classification and bundled knowledge
    #[command(
        about = "Inspect RepoPilot classification and bundled knowledge",
        long_about = "Advanced diagnostics for rule authors and power users.\n\n\
Use `inspect explain` to understand file context classification and rule decisions.\n\
Use `inspect knowledge` to inspect the bundled Knowledge Engine catalog.",
        after_help = "EXAMPLES:\n  \
repopilot inspect explain src/main.rs\n  \
repopilot inspect explain src/main.rs --rule language.rust.panic-risk --signal rust.unwrap\n  \
repopilot inspect knowledge --section rules --format json"
    )]
    Inspect(InspectOptions),

    /// Generate a default repopilot.toml configuration file
    #[command(
        about = "Generate a default repopilot.toml configuration file",
        long_about = "Writes a repopilot.toml with all configurable thresholds set to their defaults.\n\n\
Edit the generated file to tune thresholds for your project. RepoPilot automatically\n\
reads repopilot.toml from the current working directory when running `scan`.\n\n\
Configuration precedence: CLI flags > repopilot.toml > built-in defaults.",
        after_help = "EXAMPLES:\n  \
repopilot init\n  \
repopilot init --force            # overwrite existing config\n  \
repopilot init --path ./config/repopilot.toml"
    )]
    Init(InitOptions),

    /// Diagnose RepoPilot audit readiness for a repository (alias: d)
    #[command(
        alias = "d",
        about = "Diagnose RepoPilot audit readiness",
        long_about = "Runs a lightweight audit readiness check for a repository.\n\n\
It scans the target path, reports audit scope accounting, checks whether RepoPilot\n\
configuration, .repopilotignore, baseline, Git, and GitHub workflows are present,\n\
then recommends the next command to run.",
        after_help = "EXAMPLES:\n  \
repopilot doctor .\n  \
repopilot doctor . --format json\n  \
repopilot doctor . --format markdown --output doctor.md"
    )]
    Doctor(DoctorOptions),

    /// Deprecated compatibility command; use `repopilot ai context`
    #[command(alias = "v", hide = true)]
    Vibe(AiContextOptions),

    /// Deprecated compatibility command; use `repopilot ai plan`
    #[command(alias = "h", hide = true)]
    Harden(AiPlanOptions),

    /// Deprecated compatibility command; use `repopilot ai prompt`
    #[command(alias = "p", hide = true)]
    Prompt(AiPromptOptions),

    /// Deprecated compatibility command; use `repopilot inspect explain`
    #[command(alias = "e", hide = true)]
    Explain(ExplainOptions),

    /// Deprecated compatibility command; use `repopilot inspect knowledge`
    #[command(alias = "k", hide = true)]
    Knowledge(KnowledgeOptions),
}

#[derive(Args)]
pub struct BaselineOptions {
    #[command(subcommand)]
    pub command: BaselineCommands,
}

#[derive(Subcommand)]
pub enum BaselineCommands {
    /// Scan a path and store the current findings as accepted debt
    #[command(
        about = "Scan a path and store the current findings as accepted debt",
        long_about = "Runs a full scan and writes all current findings to a baseline file.\n\n\
Future scans with `--baseline` will mark each matching finding as `existing` and\n\
flag only genuinely new findings. This lets CI gate on `--fail-on new-high` without\n\
failing on pre-existing issues.\n\n\
By default writes to .repopilot/baseline.json and creates the directory if needed.\n\
Existing baseline files are not overwritten unless you pass `--force`.\n\n\
Refresh the baseline only when the team explicitly accepts the current findings\n\
as technical debt — not as a way to silence CI.",
        after_help = "EXAMPLES:\n  \
repopilot baseline create .\n  \
repopilot baseline create . --output ./baseline.json\n  \
repopilot baseline create . --force"
    )]
    Create(BaselineCreateOptions),
}

#[derive(Args)]
pub struct BaselineCreateOptions {
    /// Path to scan before writing the baseline
    pub path: PathBuf,

    /// Baseline output path; defaults to .repopilot/baseline.json
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Overwrite an existing baseline file
    #[arg(long)]
    pub force: bool,
}

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

    /// Only render findings at or above this severity
    #[arg(long, value_enum)]
    pub min_severity: Option<SeverityArg>,

    /// Print scan and render timing to stderr
    #[arg(long)]
    pub verbose: bool,

    /// Apply threshold presets without editing repopilot.toml
    #[arg(long, value_parser = ["strict", "balanced", "lenient"])]
    pub preset: Option<String>,
}

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

    /// Exit with code 1 when in-diff findings meet this threshold
    #[arg(long, value_enum)]
    pub fail_on: Option<FailOnArg>,

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
}

#[derive(Args)]
pub struct AiOptions {
    #[command(subcommand)]
    pub command: AiCommands,
}

#[derive(Subcommand)]
pub enum AiCommands {
    /// Generate LLM-ready repository context from a scan
    #[command(
        about = "Generate LLM-ready repository context from a scan",
        after_help = "EXAMPLES:\n  \
repopilot ai context .\n  \
repopilot ai context . --focus security --budget 2k\n  \
repopilot ai context . --no-header | pbcopy"
    )]
    Context(AiContextOptions),

    /// Generate a prioritized remediation plan from scan findings
    #[command(
        about = "Generate a prioritized remediation plan from scan findings",
        after_help = "EXAMPLES:\n  \
repopilot ai plan .\n  \
repopilot ai plan . --focus security --budget 4k\n  \
repopilot ai plan . --output harden.md"
    )]
    Plan(AiPlanOptions),

    /// Generate an AI-ready remediation prompt from scan findings
    #[command(
        about = "Generate an AI-ready remediation prompt from scan findings",
        after_help = "EXAMPLES:\n  \
repopilot ai prompt .\n  \
repopilot ai prompt . --focus quality --budget 4k\n  \
repopilot ai prompt . --output prompt.md"
    )]
    Prompt(AiPromptOptions),
}

#[derive(Args)]
pub struct AiContextOptions {
    /// Path to project, folder, or file to scan
    pub path: PathBuf,

    /// Path to a repopilot.toml config file
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Limit output to one finding category
    #[arg(long, value_parser = ["security", "arch", "architecture", "quality", "framework", "all"])]
    pub focus: Option<String>,

    /// Target token budget: 2k, 4k, 8k, 16k, or a positive integer
    #[arg(long, value_parser = parse_vibe_budget)]
    pub budget: Option<usize>,

    /// Write Markdown output to a file instead of stdout
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Omit the intro header block
    #[arg(long)]
    pub no_header: bool,
}

#[derive(Args)]
pub struct AiPlanOptions {
    /// Path to project, folder, or file to scan
    pub path: PathBuf,

    /// Path to a repopilot.toml config file
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Limit output to one finding category
    #[arg(long, value_parser = ["security", "arch", "architecture", "quality", "framework", "all"])]
    pub focus: Option<String>,

    /// Target token budget: 2k, 4k, 8k, 16k, or a positive integer
    #[arg(long, value_parser = parse_vibe_budget)]
    pub budget: Option<usize>,

    /// Write Markdown output to a file instead of stdout
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Args)]
pub struct AiPromptOptions {
    /// Path to project, folder, or file to scan
    pub path: PathBuf,

    /// Path to a repopilot.toml config file
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Limit output to one finding category
    #[arg(long, value_parser = ["security", "arch", "architecture", "quality", "framework", "all"])]
    pub focus: Option<String>,

    /// Target token budget: 2k, 4k, 8k, 16k, or a positive integer
    #[arg(long, value_parser = parse_vibe_budget)]
    pub budget: Option<usize>,

    /// Write Markdown output to a file instead of stdout
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

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
