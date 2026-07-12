pub mod ai;
pub mod baseline;
pub mod cache;
pub mod init;
pub mod mcp;
pub mod review;
pub mod scan;
pub mod snapshot;

pub use ai::{AiCommands, AiOptions};
pub use baseline::{BaselineCommands, BaselineOptions};
pub use cache::{CacheCommands, CacheOptions};
pub use init::InitOptions;
pub use mcp::McpOptions;
pub use review::ReviewOptions;
pub use scan::ScanOptions;
pub use snapshot::SnapshotOptions;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Manage RepoPilot's local scan cache
    #[command(
        about = "Manage RepoPilot's local scan cache",
        long_about = "Manage RepoPilot's local cache files under .repopilot/cache.\n\n\
Cache data is local to the repository and is used by changed scans.",
        after_help = "EXAMPLES:\n  \
repopilot cache clear\n  \
repopilot cache clear ."
    )]
    Cache(CacheOptions),

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

    /// Scan a project, folder, or file for findings (alias: s)
    #[command(
        alias = "s",
        display_order = 2,
        about = "Scan a project, folder, or file for findings",
        long_about = "Walks the target path and runs all enabled audit rules:\n\n\
  Architecture  — oversized files, deep nesting, deep imports, risky barrels\n  \
Coupling      — excessive fan-out, high-instability hubs, circular dependencies\n  \
Code quality  — cyclomatic complexity, long functions, runtime-risk signals\n  \
Security      — hardcoded secrets, private keys, .env files, Django settings\n  \
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
        display_order = 1,
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
repopilot review . --since-snapshot\n  \
repopilot review . --base origin/main\n  \
repopilot review . --base origin/main --head HEAD\n  \
repopilot review . --base origin/main --format markdown --output review.md\n  \
repopilot review . --baseline .repopilot/baseline.json --fail-on new-high\n  \
repopilot review . --format json --output review.json"
    )]
    Review(ReviewOptions),

    /// Mark the repository state before a change for `review --since-snapshot`
    #[command(
        display_order = 3,
        about = "Mark the repository state before an agent or manual change",
        long_about = "Records the current HEAD commit (and whether the working tree is already\n\
dirty) to .repopilot/snapshot.json — a \"before\" marker you take right before an\n\
AI agent or a manual edit changes the repository.\n\n\
Afterwards, `repopilot review --since-snapshot` reviews everything that happened\n\
since the marker: commits made on top of it and any uncommitted edits.",
        after_help = "EXAMPLES:\n  \
repopilot snapshot                          # mark the current state\n  \
repopilot review --since-snapshot           # review everything since the marker"
    )]
    Snapshot(SnapshotOptions),

    /// Generate a local AI-ready handoff (context, plan, and guidance)
    #[command(
        about = "Generate a local AI-ready handoff (context, plan, and guidance)",
        long_about = "Turns a local scan into one assistant-ready Markdown handoff: repository\n\
context and evidence, a prioritized P0–P3 remediation plan, working rules, and a\n\
verification checklist. RepoPilot does not call AI services or upload source code.",
        after_help = "EXAMPLES:\n  \
repopilot ai context . --focus security\n  \
repopilot ai context . --budget 8k\n  \
repopilot ai context . --no-task --output ai-context.md"
    )]
    Ai(AiOptions),

    /// Generate a default repopilot.toml configuration file
    #[command(
        about = "Generate a default repopilot.toml configuration file",
        long_about = "Writes a repopilot.toml with all configurable thresholds set to their defaults.\n\n\
Edit the generated file to tune thresholds for your project. RepoPilot automatically\n\
reads repopilot.toml from the current working directory for analysis commands,\n\
including `scan`, `review`, `baseline create`, AI context, and MCP tools.\n\n\
Configuration precedence: CLI flags > repopilot.toml > built-in defaults.",
        after_help = "EXAMPLES:\n  \
repopilot init\n  \
repopilot init --force            # overwrite existing config\n  \
repopilot init --path ./config/repopilot.toml"
    )]
    Init(InitOptions),

    /// Run a local Model Context Protocol server over stdio
    #[command(
        about = "Run a local Model Context Protocol server over stdio",
        long_about = "Exposes RepoPilot to AI agents (Claude Code, Cursor, …) as MCP tools they can\n\
call directly. The server speaks JSON-RPC 2.0 over stdin/stdout; nothing is\n\
uploaded and no AI service is called — every tool runs the same local analysis as\n\
the CLI.\n\n\
Register it with your agent, for example:\n  \
claude mcp add repopilot -- repopilot mcp",
        after_help = "EXAMPLES:\n  \
repopilot mcp                               # run the stdio server (clients launch this)\n  \
claude mcp add repopilot -- repopilot mcp   # register with Claude Code"
    )]
    Mcp(McpOptions),
}
