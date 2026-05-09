use clap::{Parser, Subcommand, ValueEnum};
use repopilot::baseline::gate::FailOn;
use repopilot::findings::types::Severity;
use repopilot::output::OutputFormat;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "repopilot")]
#[command(version)]
#[command(
    about = "Local-first CLI for repository audit, architecture risk detection, baseline tracking, and CI-friendly code review.",
    long_about = "RepoPilot is a local-first CLI that scans your repository for architecture risks,\n\
code quality issues, security findings, and missing tests.\n\n\
It does not upload your repository — all analysis runs locally against files on disk.\n\n\
Use `repopilot scan` to get a full picture of a project, `repopilot review` to focus\n\
on findings introduced by changed lines, and `repopilot baseline` to suppress accepted\n\
existing debt so CI gates catch only new regressions.",
    after_help = "EXAMPLES:\n  \
repopilot init                              # generate repopilot.toml\n  \
repopilot scan .                            # scan current directory\n  \
repopilot scan . --format sarif --output repopilot.sarif\n  \
repopilot review . --base origin/main       # review changes vs main\n  \
repopilot baseline create .                 # accept current findings\n  \
repopilot scan . --baseline .repopilot/baseline.json --fail-on new-high"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

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
    Baseline {
        #[command(subcommand)]
        command: BaselineCommands,
    },

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
The scan respects .gitignore and built-in ignore rules for common build directories.\n\n\
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
repopilot scan . --max-file-loc 500 --max-directory-modules 30"
    )]
    Scan {
        /// Path to project, folder, or file
        path: PathBuf,

        /// Output format (console, json, markdown, html, sarif)
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

        /// Scan each workspace package separately and group findings by package
        #[arg(long, short = 'w')]
        workspace: bool,

        /// Only show findings at or above this severity level
        #[arg(long, value_enum)]
        min_severity: Option<SeverityArg>,

        /// Print scan phase timing breakdown after the report
        #[arg(long)]
        verbose: bool,

        /// Apply a threshold preset: strict, balanced (default), lenient
        #[arg(long, value_parser = ["strict", "balanced", "lenient"])]
        preset: Option<String>,
    },

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
    Review {
        /// Path to project, folder, or file
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Base Git ref for review diff. Without this, review compares the working tree against HEAD
        #[arg(long)]
        base: Option<String>,

        /// Head Git ref for review diff. Requires --base and defaults to HEAD when --base is set
        #[arg(long)]
        head: Option<String>,

        /// Path to a RepoPilot config file
        #[arg(long)]
        config: Option<PathBuf>,

        /// Path to a RepoPilot baseline file
        #[arg(long)]
        baseline: Option<PathBuf>,

        /// Fail with exit code 1 when in-diff findings meet the selected threshold
        #[arg(long, value_enum)]
        fail_on: Option<FailOnArg>,

        /// Output format (console, json, markdown)
        #[arg(long, value_enum, default_value = "console")]
        format: CompareOutputFormatArg,

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

        /// Only show findings at or above this severity level
        #[arg(long, value_enum)]
        min_severity: Option<SeverityArg>,
    },

    /// Generate an LLM-ready context from a scan — paste into Claude Code, Cursor, or ChatGPT (alias: v)
    #[command(
        alias = "v",
        about = "Generate an LLM-ready context from a scan",
        long_about = "Scans the repository and formats findings as structured markdown optimised\n\
for pasting into Claude Code, Cursor, ChatGPT, or any LLM assistant.\n\n\
The output includes a risk summary, tech stack detection, findings grouped by\n\
category (Security, Architecture, Code Quality, Testing, Framework), evidence\n\
snippets, fix recommendations, and a token-count estimate.\n\n\
Use `--focus` to limit output to a single category, and `--budget` to control\n\
how many tokens the output targets.",
        after_help = "EXAMPLES:\n  \
repopilot vibe .\n  \
repopilot vibe . --focus security\n  \
repopilot vibe . --budget 8k\n  \
repopilot vibe . --output vibe.md\n  \
repopilot vibe . --no-header | pbcopy"
    )]
    Vibe {
        /// Path to project, folder, or file
        path: PathBuf,

        /// Path to a RepoPilot config file
        #[arg(long)]
        config: Option<PathBuf>,

        /// Limit output to a single category: security, arch, architecture, quality, framework, all
        #[arg(long, value_parser = ["security", "arch", "architecture", "quality", "framework", "all"])]
        focus: Option<String>,

        /// Target token budget for output: 2k, 4k, 8k, 16k (default: 4k)
        #[arg(long, value_parser = parse_vibe_budget)]
        budget: Option<usize>,

        /// Write output to a file instead of stdout
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Omit the intro header block (useful for piping into an LLM API)
        #[arg(long)]
        no_header: bool,
    },

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
    Create {
        /// Path to project, folder, or file
        path: PathBuf,

        /// Write baseline to a custom path (default: .repopilot/baseline.json)
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
    Sarif,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum CompareOutputFormatArg {
    Console,
    Json,
    Markdown,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum SeverityArg {
    Info,
    Low,
    Medium,
    High,
    Critical,
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

impl From<CompareOutputFormatArg> for OutputFormat {
    fn from(format: CompareOutputFormatArg) -> Self {
        match format {
            CompareOutputFormatArg::Console => OutputFormat::Console,
            CompareOutputFormatArg::Json => OutputFormat::Json,
            CompareOutputFormatArg::Markdown => OutputFormat::Markdown,
        }
    }
}

impl From<OutputFormatArg> for OutputFormat {
    fn from(format: OutputFormatArg) -> Self {
        match format {
            OutputFormatArg::Console => OutputFormat::Console,
            OutputFormatArg::Html => OutputFormat::Html,
            OutputFormatArg::Json => OutputFormat::Json,
            OutputFormatArg::Markdown => OutputFormat::Markdown,
            OutputFormatArg::Sarif => OutputFormat::Sarif,
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

fn parse_vibe_budget(value: &str) -> Result<usize, String> {
    let tokens = match value {
        "2k" => 2048,
        "4k" => 4096,
        "8k" => 8192,
        "16k" => 16384,
        other => other
            .parse::<usize>()
            .map_err(|_| "expected 2k, 4k, 8k, 16k, or a positive token count".to_string())?,
    };

    if tokens == 0 {
        return Err("budget must be greater than zero".to_string());
    }

    Ok(tokens)
}
