use crate::baseline::diff::BaselineStatus;
use crate::findings::types::Finding;
use crate::review::diff::ChangedFile;
use crate::review::model::ReviewReport;
use crate::review::proof::{ChangeProof, ChangeProofVerdict, ProofObligations};
use crate::review::readiness::MergeReadinessRecord;
use crate::verification::VerificationOutcome;

pub(super) fn verification_duration_evidence(outcome: &VerificationOutcome) -> String {
    if outcome.reused {
        format!("original run {} ms", outcome.duration_ms)
    } else {
        format!("{} ms", outcome.duration_ms)
    }
}

pub(super) fn verification_diagnostics_evidence(outcome: &VerificationOutcome) -> Option<String> {
    let diagnostics = outcome.diagnostics.as_ref()?;
    if diagnostics.complete && !diagnostics.entries.is_empty() {
        return Some(
            diagnostics
                .entries
                .iter()
                .map(|entry| entry.key.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        );
    }
    diagnostics
        .limitation
        .as_deref()
        .map(|limitation| format!("unavailable: {limitation}"))
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

pub(super) fn change_proof_headline(report: &ReviewReport, proof: &ChangeProof) -> &'static str {
    match proof.verdict {
        ChangeProofVerdict::Broken => "A supported contract appears broken in the changed scope.",
        ChangeProofVerdict::Review => {
            "Review the listed evidence, coverage limits, and required checks."
        }
        ChangeProofVerdict::Verified => "The assessed scope satisfies the selected proof policy.",
        ChangeProofVerdict::NotAssessed if report.changed_files.is_empty() => {
            "No changed files were available for assessment."
        }
        ChangeProofVerdict::NotAssessed => "No analyzable files were available for assessment.",
    }
}

pub(super) fn change_proof_policy_summary(report: &ReviewReport) -> String {
    let configured = report.verification_policy.configured.len();
    let selected = report.verification_policy.selected.len();
    if selected == 0 {
        if report.verification.is_empty() {
            format!("none selected ({configured} configured)")
        } else {
            format!(
                "recorded outcomes ({} check(s); no configured policy)",
                report.verification.len()
            )
        }
    } else {
        format!("{selected} selected ({configured} configured)")
    }
}

pub(super) fn legacy_readiness_summary(
    report: &ReviewReport,
    readiness: &MergeReadinessRecord,
) -> String {
    let label = readiness.verdict.label();
    if report.changed_files.is_empty() {
        format!("{label} (compatibility field; no changed scope assessed)")
    } else {
        label.to_string()
    }
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
