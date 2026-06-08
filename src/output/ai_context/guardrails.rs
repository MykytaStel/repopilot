//! The "how to work" guardrails embedded in the `ai context` task block — a
//! condensed version of the operating rules a coding assistant should follow
//! when acting on RepoPilot findings. Emitted only when the task block is on, so
//! agents that bring their own instructions (e.g. over MCP with `no_task`) get a
//! lean, fact-only context.

use std::fmt::Write as FmtWrite;

pub(super) fn render_working_rules(out: &mut String) {
    let _ = writeln!(
        out,
        "\n## How To Work\n\n\
- Fix the highest-risk findings first (P0, then P1); leave low-signal cleanup for a separate change.\n\
- Make the smallest safe edits that resolve a finding; do not rewrite broad architecture to move a metric.\n\
- Work within the existing code style, module boundaries, and public APIs; preserve serialized formats and documented behavior.\n\
- Inspect the cited files and nearby tests before editing, and explain any finding you judge to be a false positive.\n\
- Add or update tests whenever behavior changes or a regression is plausible.\n\
- Do not upload source code, call external services, or refresh a baseline to hide newly introduced risk."
    );
}
