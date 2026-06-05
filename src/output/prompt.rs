use crate::output::ai_context::{
    AiContextRenderOptions, AiFocusCategory, DEFAULT_TOKEN_BUDGET, project_name,
    render as render_ai_context,
};
use crate::scan::types::ScanSummary;
use std::fmt::Write as FmtWrite;

pub struct PromptOptions {
    pub focus: Option<AiFocusCategory>,
    pub budget_tokens: usize,
}

impl Default for PromptOptions {
    fn default() -> Self {
        Self {
            focus: None,
            budget_tokens: DEFAULT_TOKEN_BUDGET,
        }
    }
}

pub fn render(summary: &ScanSummary, opts: &PromptOptions) -> String {
    let project_name = project_name(summary);

    let mut out = String::new();
    let _ = writeln!(out, "# RepoPilot Remediation Prompt - {project_name}\n");
    let _ = writeln!(
        out,
        "You are an AI coding assistant working inside this repository. Use the RepoPilot context below as evidence, then make the smallest safe code changes that reduce the highest repository risk first."
    );
    render_scope(&mut out, opts);
    render_repository_facts(&mut out, summary);
    let _ = writeln!(
        out,
        "\n## Non-goals\n\n- Do not try to fix every RepoPilot finding in one change.\n- Do not add new commands, flags, output formats, or report schema fields unless explicitly requested.\n- Do not rewrite broad architecture just to reduce a metric.\n- Do not refresh baselines to hide newly introduced risk."
    );
    let _ = writeln!(
        out,
        "\n## Operating Rules\n\n- Work from the existing code style and module boundaries.\n- Do not upload source code or call external services.\n- Do not rewrite unrelated files or revert user changes.\n- Preserve public APIs, serialized formats, CLI flags, and documented behavior unless a finding explicitly requires changing them.\n- Prefer narrow, reviewable edits over broad rewrites.\n- Add or update tests whenever behavior changes or a regression is plausible."
    );
    let _ = writeln!(
        out,
        "\n## Triage Order\n\n1. Fix Critical findings first.\n2. Fix High security findings next.\n3. Fix High architecture or framework findings that increase blast radius.\n4. Fix Medium maintainability findings only when they are local, obvious, or unblock higher-risk work.\n5. Leave low-signal cleanup for a separate change unless it is already touched by the fix."
    );
    let _ = writeln!(
        out,
        "\n## Implementation Loop\n\n1. Inspect the cited files and nearby tests before editing.\n2. State the concrete fixes you will make and the behavior each fix protects.\n3. Make the code changes.\n4. Run the narrowest relevant tests or checks first.\n5. Run broader checks when practical.\n6. If a finding is a false positive, explain why and add a focused regression test when possible."
    );
    render_required_checks(&mut out, summary);
    let _ = writeln!(
        out,
        "\n## Verification Contract\n\n- Report exact commands run and whether they passed.\n- If a command was skipped, explain the blocker.\n- Re-run `repopilot scan .` or the relevant focused scan when the fix targets RepoPilot findings."
    );
    let _ = writeln!(
        out,
        "\n## Expected Output\n\n- Top risks addressed.\n- Files changed and why.\n- Verification results.\n- Remaining risk or follow-up, if any.\n"
    );
    let _ = writeln!(out, "## RepoPilot Context\n");

    const PROMPT_PREFIX_OVERHEAD_TOKENS: usize = 200;
    let context_budget = opts
        .budget_tokens
        .saturating_sub(PROMPT_PREFIX_OVERHEAD_TOKENS);
    let ai_context = render_ai_context(
        summary,
        &AiContextRenderOptions {
            focus: opts.focus.clone(),
            budget_tokens: context_budget,
            no_header: false,
            no_task: true,
        },
    );
    out.push_str(&ai_context);
    out
}

fn render_scope(out: &mut String, opts: &PromptOptions) {
    let focus = opts
        .focus
        .as_ref()
        .map(focus_label)
        .unwrap_or("all RepoPilot findings");
    let _ = writeln!(
        out,
        "\n## Scope\n\n- Focus: {focus}.\n- Token budget: approximately {} tokens for the embedded RepoPilot context.",
        opts.budget_tokens
    );
}

fn render_repository_facts(out: &mut String, summary: &ScanSummary) {
    let languages = if summary.metrics.languages.is_empty() {
        "unknown".to_string()
    } else {
        summary
            .metrics
            .languages
            .iter()
            .map(|language| format!("{} ({})", language.name, language.files_analyzed))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let _ = writeln!(
        out,
        "\n## Repository Facts\n\n- Files analyzed: {}.\n- Non-empty lines: {}.\n- Languages: {languages}.\n- Visible RepoPilot findings: {}.\n- RepoPilot recommendations below are guidance derived from local evidence; verify each finding in code before editing.",
        summary.metrics.files_analyzed,
        summary.metrics.non_empty_lines,
        summary.artifacts.findings.len()
    );
}

fn render_required_checks(out: &mut String, summary: &ScanSummary) {
    let mut checks = vec![
        "repopilot scan .".to_string(),
        "repopilot review . --base origin/main".to_string(),
    ];

    if has_language(summary, "rust") {
        checks.extend([
            "cargo fmt --all -- --check".to_string(),
            "cargo clippy --all-targets --all-features -- -D warnings".to_string(),
            "cargo test --all".to_string(),
        ]);
    }

    let _ = writeln!(out, "\n## Required Checks\n");
    for check in checks {
        let _ = writeln!(out, "- `{check}`");
    }
}

fn has_language(summary: &ScanSummary, expected: &str) -> bool {
    summary
        .metrics
        .languages
        .iter()
        .any(|language| language.name.eq_ignore_ascii_case(expected))
}

fn focus_label(focus: &AiFocusCategory) -> &'static str {
    match focus {
        AiFocusCategory::Security => "security findings",
        AiFocusCategory::Architecture => "architecture findings",
        AiFocusCategory::Quality => "code quality and testing findings",
        AiFocusCategory::Framework => "framework findings",
        AiFocusCategory::All => "all RepoPilot findings",
    }
}
