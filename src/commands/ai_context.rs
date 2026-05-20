use crate::cli::ai::AiContextOptions;
use crate::commands::llm::{LlmCommandArgs, run_markdown_command};
use repopilot::output::ai_context::{AiContextRenderOptions, render_with_breakdown};
use std::io::IsTerminal;

pub fn run(options: AiContextOptions) -> Result<(), Box<dyn std::error::Error>> {
    let AiContextOptions {
        path,
        config,
        focus,
        budget,
        output,
        no_header,
        no_task,
        show_breakdown,
    } = options;

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
            let opts = AiContextRenderOptions {
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
