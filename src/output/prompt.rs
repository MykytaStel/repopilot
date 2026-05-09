use crate::output::vibe::{VibeCategory, VibeOptions, render as render_vibe};
use crate::scan::types::ScanSummary;
use std::fmt::Write as FmtWrite;

pub struct PromptOptions {
    pub focus: Option<VibeCategory>,
    pub budget_tokens: usize,
}

impl Default for PromptOptions {
    fn default() -> Self {
        Self {
            focus: None,
            budget_tokens: 4096,
        }
    }
}

pub fn render(summary: &ScanSummary, opts: &PromptOptions) -> String {
    let project_name = summary
        .root_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("project");

    let mut out = String::new();
    let _ = writeln!(out, "# RepoPilot Remediation Prompt - {project_name}\n");
    let _ = writeln!(
        out,
        "You are an AI coding assistant working inside this repository. Use the RepoPilot context below to make a small, reviewable remediation plan, then implement the highest-risk fixes first."
    );
    let _ = writeln!(
        out,
        "\nConstraints:\n- Do not call external services or upload source code.\n- Preserve public APIs unless a finding explicitly requires a change.\n- Prefer tests for changed behavior.\n- After edits, run the narrowest relevant checks first, then the full project checks when practical."
    );
    let _ = writeln!(
        out,
        "\nExpected response:\n1. State the top risks you will fix.\n2. Make the code changes.\n3. Summarize changed files and verification results.\n"
    );
    let _ = writeln!(out, "## RepoPilot Context\n");

    let vibe = render_vibe(
        summary,
        &VibeOptions {
            focus: opts.focus.clone(),
            budget_tokens: opts.budget_tokens,
            no_header: false,
        },
    );
    out.push_str(&vibe);
    out
}
