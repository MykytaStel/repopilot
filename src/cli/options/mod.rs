pub mod ai;
pub mod baseline;
pub mod compare;
pub mod doctor;
pub mod inspect;
pub mod review;
pub mod scan;

pub use ai::{AiCommands, AiOptions};
pub use baseline::{BaselineCommands, BaselineOptions};
pub use compare::CompareOptions;
pub use doctor::{DoctorOptions, InitOptions};
pub use inspect::{InspectCommands, InspectOptions};
pub use review::ReviewOptions;
pub use scan::ScanOptions;

use clap::Subcommand;

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
}
