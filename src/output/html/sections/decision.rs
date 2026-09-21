use crate::output::decision_summary::scan_decision_summary;
use crate::output::decision_view::{decision_limits, decision_next_action};

pub(super) fn render_decision_section(summary: &ScanSummary) -> String {
    let decision = scan_decision_summary(summary);
    let mut output = String::new();
    writeln!(
        output,
        r#"<section class="panel decision-summary" aria-labelledby="decision-heading">"#
    )
    .unwrap();
    writeln!(
        output,
        "  <h2 id=\"decision-heading\">Decision: {}</h2>",
        escape_html(decision.verdict.label())
    )
    .unwrap();
    writeln!(
        output,
        "  <p><strong>Why:</strong> {}</p>",
        escape_html(&decision.headline)
    )
    .unwrap();
    writeln!(
        output,
        "  <p><strong>Limits:</strong> {}</p>",
        escape_html(&decision_limits(&decision))
    )
    .unwrap();
    writeln!(
        output,
        "  <p><strong>Next action:</strong> {}</p>",
        escape_html(decision_next_action(&decision))
    )
    .unwrap();
    writeln!(
        output,
        "  <p><strong>Decision inputs:</strong> {} finding(s), P0 {}, P1 {}, {} verification plan(s)</p>",
        decision.findings, decision.p0, decision.p1, decision.verification_plans
    )
    .unwrap();
    output.push_str("</section>\n");
    output
}
