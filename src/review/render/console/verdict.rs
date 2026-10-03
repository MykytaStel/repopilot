//! The decision line, its reasons, and one next action.

use crate::review::decision::{ReviewDecision, headline_reasons};
use crate::review::model::ReviewReport;
use crate::review::proof::ChangeProof;
use std::fmt::Write;

const MAX_HEADER_REASONS: usize = 5;

pub(super) fn render(
    output: &mut String,
    report: &ReviewReport,
    decision: &ReviewDecision,
    proof: &ChangeProof,
) {
    let unverified_pass = decision.is_unverified_pass(proof);
    if unverified_pass {
        let _ = writeln!(output, "Decision: PASS (not verified)");
    } else {
        let _ = writeln!(
            output,
            "Decision: {} (Change Proof: {})",
            decision.verdict.label(),
            proof.verdict.label()
        );
    }
    let _ = writeln!(output, "Meaning: {}", decision.meaning);
    let reasons = if unverified_pass {
        Vec::new()
    } else {
        headline_reasons(report, proof)
    };
    if !reasons.is_empty() {
        output.push_str("Reasons:\n");
        for reason in reasons.iter().take(MAX_HEADER_REASONS) {
            let _ = writeln!(output, "  - {reason}");
        }
        if reasons.len() > MAX_HEADER_REASONS {
            let _ = writeln!(
                output,
                "  ... {} more reason(s) in --format json",
                reasons.len() - MAX_HEADER_REASONS
            );
        }
    }
    let _ = writeln!(output, "Next action: {}", decision.next_action);
}
