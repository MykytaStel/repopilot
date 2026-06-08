mod args;
mod options;

pub use args::{
    ColorArg, CompareOutputFormatArg, ConfidenceArg, FailOnArg, GraphOutputFormatArg,
    KnowledgeSectionArg, MaxFindingsArg, OutputFormatArg, PriorityArg, ReviewFailOnArg,
    ReviewScopeArg, RuleLifecycleArg, ScanOutputStyleArg, ScanProfileArg, SeverityArg,
    SignalSourceArg, parse_byte_size, parse_max_findings, parse_token_budget,
};
pub use options::*;

use clap::Parser;

#[derive(Parser)]
#[command(name = "repopilot")]
#[command(version)]
#[command(
    about = "Local-first CLI for reviewing Git changes, security boundaries, and blast radius before merge.",
    long_about = "RepoPilot is a review-first local CLI for understanding what changed before merge.\n\
It surfaces security boundaries, behavioral and algorithmic shifts, taint-lite flows,\n\
changed-code findings, and blast radius.\n\n\
It does not upload your repository — all analysis runs locally against files on disk.\n\n\
Start with `repopilot review`. Use `repopilot scan` for repository-wide audits,\n\
`repopilot baseline` for gradual CI adoption, and `repopilot ai` for local\n\
AI-ready remediation context.",
    after_help = "EXAMPLES:\n  \
repopilot review .                          # review working-tree changes\n  \
repopilot review . --base origin/main       # review changes vs main\n  \
repopilot snapshot                          # mark state before an agent change\n  \
repopilot review . --since-snapshot         # review the complete agent change\n  \
repopilot scan .                            # run a repository-wide audit\n  \
repopilot baseline create .                 # accept current findings\n  \
repopilot ai context . --focus security     # generate AI-ready context"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}
