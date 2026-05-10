mod args;

pub use args::{
    parse_vibe_budget, CompareOutputFormatArg, FailOnArg, OutputFormatArg, SeverityArg,
};

use clap::{Parser, Subcommand};
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
        before: std::path::PathBuf,
        after: std::path::PathBuf,
        #[arg(long, value_enum, default_value = "console")]
        format: CompareOutputFormatArg,
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
        path: PathBuf,
        #[arg(long, value_enum)]
        format: Option<OutputFormatArg>,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        baseline: Option<PathBuf>,
        #[arg(long, value_enum)]
        fail_on: Option<FailOnArg>,
        #[arg(long)]
        max_file_loc: Option<usize>,
        #[arg(long)]
        max_directory_modules: Option<usize>,
        #[arg(long)]
        max_directory_depth: Option<usize>,
        #[arg(long, short = 'w')]
        workspace: bool,
        #[arg(long, value_enum)]
        min_severity: Option<SeverityArg>,
        #[arg(long)]
        verbose: bool,
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
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        base: Option<String>,
        #[arg(long)]
        head: Option<String>,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        baseline: Option<PathBuf>,
        #[arg(long, value_enum)]
        fail_on: Option<FailOnArg>,
        #[arg(long, value_enum, default_value = "console")]
        format: CompareOutputFormatArg,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        max_file_loc: Option<usize>,
        #[arg(long)]
        max_directory_modules: Option<usize>,
        #[arg(long)]
        max_directory_depth: Option<usize>,
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
        path: PathBuf,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long, value_parser = ["security", "arch", "architecture", "quality", "framework", "all"])]
        focus: Option<String>,
        #[arg(long, value_parser = parse_vibe_budget)]
        budget: Option<usize>,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        no_header: bool,
    },

    /// Generate a prioritized remediation plan from scan findings (alias: h)
    #[command(
        alias = "h",
        about = "Generate a prioritized remediation plan from scan findings",
        long_about = "Scans the repository and formats findings as a deterministic hardening plan.\n\n\
The output is Markdown with P0/P1/P2/P3 priorities, locations, rule IDs,\n\
recommendations, and verification commands. It runs fully locally and does not\n\
call an AI service.",
        after_help = "EXAMPLES:\n  \
repopilot harden .\n  \
repopilot harden . --focus security\n  \
repopilot harden . --budget 8k\n  \
repopilot harden . --output harden.md"
    )]
    Harden {
        path: PathBuf,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long, value_parser = ["security", "arch", "architecture", "quality", "framework", "all"])]
        focus: Option<String>,
        #[arg(long, value_parser = parse_vibe_budget)]
        budget: Option<usize>,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Generate an AI-ready remediation prompt from scan findings (alias: p)
    #[command(
        alias = "p",
        about = "Generate an AI-ready remediation prompt from scan findings",
        long_about = "Scans the repository and emits a Markdown prompt with local RepoPilot context.\n\n\
Use it when you want to paste a single instruction block into Claude Code,\n\
Cursor, ChatGPT, or another coding assistant. RepoPilot only generates the\n\
prompt locally; it does not call an AI service.",
        after_help = "EXAMPLES:\n  \
repopilot prompt .\n  \
repopilot prompt . --focus security --budget 2k\n  \
repopilot prompt . --output prompt.md"
    )]
    Prompt {
        path: PathBuf,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long, value_parser = ["security", "arch", "architecture", "quality", "framework", "all"])]
        focus: Option<String>,
        #[arg(long, value_parser = parse_vibe_budget)]
        budget: Option<usize>,
        #[arg(short, long)]
        output: Option<PathBuf>,
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
        #[arg(long)]
        force: bool,
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
        path: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        force: bool,
    },
}
