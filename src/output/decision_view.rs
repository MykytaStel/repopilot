use super::decision_summary::{DecisionSummary, DecisionVerdict};

/// Returns the bounded evidence context that should accompany a human-facing decision.
///
/// This describes the scope without implying that static analysis proves runtime behaviour
/// or human readiness.
pub(crate) fn decision_limits(summary: &DecisionSummary) -> String {
    if summary.verdict == DecisionVerdict::NotAssessed {
        return "No analyzable files were available, so no repository health claim can be made."
            .to_string();
    }

    let mut limits =
        "Static evidence covers only analyzed files in the selected visibility profile."
            .to_string();
    if !summary.reasons.is_empty() {
        limits.push(' ');
        limits.push_str(&summary.reasons.join(" "));
    }
    limits
}

/// Returns the next concrete action for a human reading a scan report.
pub(crate) fn decision_next_action(summary: &DecisionSummary) -> &'static str {
    match summary.verdict {
        DecisionVerdict::Block => {
            "Resolve the blocking evidence or scan errors, then rerun the scan."
        }
        DecisionVerdict::Review => {
            "Review the visible findings and their evidence before treating the repository as ready."
        }
        DecisionVerdict::NotAssessed => "Expand the analyzable scope, then rerun the scan.",
        DecisionVerdict::Pass
            if summary
                .reasons
                .iter()
                .any(|reason| reason.contains("strict-only")) =>
        {
            "Run with --profile strict to inspect hidden suggestions before treating this scan as fully clean."
        }
        DecisionVerdict::Pass => {
            "Proceed with the normal review; no visible findings require action in this profile."
        }
    }
}
