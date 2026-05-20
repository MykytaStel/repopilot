use crate::commands::llm::{LlmCommandArgs, run_markdown_command};
use repopilot::output::ai_plan::{AiPlanOptions, render as render_ai_plan};
use std::path::PathBuf;

pub fn run(
    path: PathBuf,
    config: Option<PathBuf>,
    focus: Option<String>,
    budget: Option<usize>,
    output: Option<PathBuf>,
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
            render_ai_plan(
                summary,
                &AiPlanOptions {
                    focus,
                    budget_tokens,
                },
            )
        },
    )
}
