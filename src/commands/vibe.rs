use crate::commands::llm::{LlmCommandArgs, run_markdown_command};
use repopilot::output::vibe::{VibeOptions, render_with_breakdown};
use std::io::IsTerminal;
use std::path::PathBuf;

pub fn run(
    path: PathBuf,
    config: Option<PathBuf>,
    focus: Option<String>,
    budget: Option<usize>,
    output: Option<PathBuf>,
    no_header: bool,
    no_task: bool,
    show_breakdown: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Show breakdown when explicitly requested or when stdout is a terminal
    // (piping to pbcopy/clip suppresses it automatically).
    let should_show_breakdown = show_breakdown || std::io::stdout().is_terminal();

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
                no_task,
            };
            let (content, breakdown) = render_with_breakdown(summary, &opts);
            if should_show_breakdown {
                breakdown.render_stderr();
            }
            content
        },
    )
}
