use crate::commands::llm::{LlmCommandArgs, run_markdown_command};
use repopilot::output::vibe::{VibeOptions, render as render_vibe};
use std::path::PathBuf;

pub fn run(
    path: PathBuf,
    config: Option<PathBuf>,
    focus: Option<String>,
    budget: Option<usize>,
    output: Option<PathBuf>,
    no_header: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    run_markdown_command(
        LlmCommandArgs {
            path,
            config,
            focus,
            budget,
            output,
        },
        |summary, focus, budget_tokens| {
            let opts = VibeOptions {
                focus,
                budget_tokens,
                no_header,
            };
            render_vibe(summary, &opts)
        },
    )
}
