use crate::cli::ai::AiContextOptions;
use crate::commands::llm::{LlmCommandArgs, run_markdown_command};
use repopilot::output::ai_context::{
    AiContextRenderOptions, render_json, render_with_facts_summary_and_breakdown,
};
use std::io::IsTerminal;

pub fn run(options: AiContextOptions) -> Result<(), Box<dyn std::error::Error>> {
    let AiContextOptions {
        path,
        config,
        focus,
        budget,
        output,
        format,
        no_header,
        no_task,
        show_breakdown,
    } = options;

    let want_json = format == "json";
    // Show breakdown when explicitly requested or when stdout is a terminal
    // (piping to pbcopy/clip suppresses it automatically). JSON output stays
    // machine-clean, so the breakdown is never mixed in.
    let should_show_breakdown = !want_json && (show_breakdown || std::io::stdout().is_terminal());

    run_markdown_command(
        LlmCommandArgs {
            path,
            config,
            focus,
            budget,
            output,
        },
        |summary, facts_summary, focus, budget_tokens| {
            let opts = AiContextRenderOptions {
                focus,
                budget_tokens,
                no_header,
                no_task,
            };
            if want_json {
                return render_json(summary, facts_summary, &opts);
            }
            let (content, breakdown) =
                render_with_facts_summary_and_breakdown(summary, facts_summary, &opts);
            if should_show_breakdown {
                breakdown.render_stderr();
            }
            content
        },
    )
}
