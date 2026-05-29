mod args;
mod options;

pub use args::{
    ColorArg, CompareOutputFormatArg, ConfidenceArg, FailOnArg, GraphOutputFormatArg,
    KnowledgeSectionArg, MaxFindingsArg, OutputFormatArg, PriorityArg, RuleLifecycleArg,
    ScanOutputStyleArg, ScanProfileArg, SeverityArg, SignalSourceArg, parse_byte_size,
    parse_max_findings, parse_token_budget,
};
pub use options::*;

use clap::Parser;

#[derive(Parser)]
#[command(name = "repopilot")]
#[command(version)]
#[command(
    about = "Local-first CLI for repository audit, architecture risk detection, baseline tracking, and CI-friendly code review.",
    long_about = "RepoPilot is a local-first CLI that scans your repository for architecture risks,\n\
code quality issues, security findings, and missing tests.\n\n\
It does not upload your repository — all analysis runs locally against files on disk.\n\n\
Use `repopilot scan` for project audits, `repopilot review` for changed-code review,\n\
`repopilot baseline` for CI adoption, and `repopilot ai` for local AI-ready remediation context.",
    after_help = "EXAMPLES:\n  \
repopilot init                              # generate repopilot.toml\n  \
repopilot scan .                            # scan current directory\n  \
repopilot scan . --format sarif --output repopilot.sarif\n  \
repopilot review . --base origin/main       # review changes vs main\n  \
repopilot baseline create .                 # accept current findings\n  \
repopilot ai context . --focus security     # generate AI-ready context"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}
