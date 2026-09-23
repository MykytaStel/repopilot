use crate::output::decision_summary::scan_decision_summary;
use crate::output::decision_view::{decision_limits, decision_next_action};

pub(super) fn render_decision(output: &mut String, summary: &ScanSummary) {
    let decision = scan_decision_summary(summary);

    output.push_str("## Decision\n\n");
    writeln!(output, "- **Decision:** `{}`", decision.verdict.label()).unwrap();
    writeln!(output, "- **Why:** {}", decision.headline).unwrap();
    writeln!(output, "- **Limits:** {}", decision_limits(&decision)).unwrap();
    let next_action = decision_next_action(&decision).replace("--profile strict", "`--profile strict`");
    writeln!(output, "- **Next action:** {next_action}").unwrap();
    writeln!(
        output,
        "- **Decision inputs:** {} finding(s), P0 {}, P1 {}, {} verification plan(s)",
        decision.findings, decision.p0, decision.p1, decision.verification_plans
    )
    .unwrap();
    output.push('\n');
}
