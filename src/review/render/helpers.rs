use crate::baseline::diff::BaselineStatus;
use crate::findings::types::Finding;
use crate::review::diff::ChangedFile;
use crate::review::model::ReviewReport;
use crate::review::proof::ProofObligations;
use crate::verification::VerificationOutcome;

pub(super) fn verification_duration_evidence(outcome: &VerificationOutcome) -> String {
    if outcome.reused {
        format!("original run {} ms", outcome.duration_ms)
    } else {
        format!("{} ms", outcome.duration_ms)
    }
}

pub(super) fn verification_proof_summary(
    report: &ReviewReport,
    obligations: ProofObligations,
) -> String {
    if report.verification.is_empty() && obligations.applicable == 0 {
        return "none selected; no verification evidence".to_string();
    }

    let revision = if report
        .verification
        .iter()
        .all(|outcome| outcome.revision_compatible)
    {
        "revision-compatible"
    } else {
        "revision-incompatible"
    };
    format!(
        "{} passed, {} failed, {} unavailable, {} unselected, {} stale ({revision})",
        obligations.satisfied,
        obligations.failed,
        obligations.unavailable,
        obligations.unselected,
        obligations.stale,
    )
}

pub(super) fn status_for_finding(
    report: &ReviewReport,
    finding: &Finding,
) -> Option<BaselineStatus> {
    report
        .summary
        .artifacts
        .findings
        .iter()
        .position(|candidate| candidate == finding)
        .and_then(|index| report.finding_status(index))
        .and_then(|status| status.baseline_status)
}

pub(super) fn render_ranges_suffix(file: &ChangedFile) -> String {
    let ranges = render_ranges(file);
    if ranges == "n/a" {
        String::new()
    } else {
        format!(" ({ranges})")
    }
}

pub(super) fn render_ranges(file: &ChangedFile) -> String {
    if file.ranges.is_empty() {
        return "n/a".to_string();
    }

    file.ranges
        .iter()
        .map(|range| {
            if range.start == range.end {
                range.start.to_string()
            } else {
                format!("{}-{}", range.start, range.end)
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}
