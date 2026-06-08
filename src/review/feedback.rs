use crate::findings::feedback::{ReviewSuppression, validate_local_feedback};
use crate::review::signals::tiered::{ReviewSignal, TieredSignals};
use crate::scan::types::ScanSummary;
use std::path::Path;

pub(super) fn apply_review_feedback(
    tiered: &mut TieredSignals,
    summary: &mut ScanSummary,
    repo_root: &Path,
) {
    if summary.local_feedback.is_none() {
        return;
    }
    let Ok(validation) = validate_local_feedback(repo_root) else {
        return;
    };
    let mut suppressed = 0;
    for signal in tiered.iter_mut() {
        let Some(suppression) = validation
            .review_suppressions
            .iter()
            .find(|suppression| suppression_matches(suppression, signal))
        else {
            continue;
        };
        signal.suppressed = true;
        signal.gate_eligible = false;
        signal.suppression_reason = suppression.reason.clone();
        suppressed += 1;
    }
    if let Some(report) = summary.local_feedback.as_mut() {
        report.suppressed_review_signals_count = suppressed;
    }
}

fn suppression_matches(suppression: &ReviewSuppression, signal: &ReviewSignal) -> bool {
    if suppression.kind != signal.kind || suppression_expired(suppression) {
        return false;
    }
    globset::Glob::new(&suppression.path)
        .map(|glob| glob.compile_matcher().is_match(&signal.path))
        .unwrap_or(false)
}

fn suppression_expired(suppression: &ReviewSuppression) -> bool {
    suppression
        .expires
        .as_deref()
        .and_then(|value| chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())
        .is_some_and(|date| date < chrono::Utc::now().date_naive())
}
