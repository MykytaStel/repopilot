use crate::commands::{CliExit, EXIT_USAGE};
use repopilot::review::diff::OwnedDiffTarget;
use repopilot::review::model::ReviewReport;
use repopilot::scan::session::AnalysisSession;
use repopilot::verification::{
    CancellationToken, VerificationExecutionEvent, VerificationStatus, run_checks_observed,
    select_checks, validate_review_target,
};
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ReviewVerificationEvent {
    Started {
        check_id: String,
        index: usize,
        total: usize,
    },
    Completed {
        check_id: String,
        index: usize,
        total: usize,
        status: VerificationStatus,
    },
}

pub(super) fn run_selected(
    selected: &[String],
    session: &AnalysisSession,
    target: &OwnedDiffTarget,
    report: &mut ReviewReport,
) -> Result<(), Box<dyn std::error::Error>> {
    run_selected_with_context(
        selected,
        session,
        target,
        report,
        &CancellationToken::new(),
        &mut |_| {},
    )
}

pub(super) fn run_selected_with_context(
    selected: &[String],
    session: &AnalysisSession,
    target: &OwnedDiffTarget,
    report: &mut ReviewReport,
    cancellation: &CancellationToken,
    observer: &mut dyn FnMut(ReviewVerificationEvent),
) -> Result<(), Box<dyn std::error::Error>> {
    if selected.is_empty() {
        return Ok(());
    }
    let config = session.repo_config();
    validate_review_target(session.workspace_root(), target, &config.scan.ignore)
        .map_err(usage_error)?;
    let checks = select_checks(
        session.workspace_root(),
        &config.verification.checks,
        selected,
    )
    .map_err(usage_error)?;
    let mut evidence_paths = report
        .changed_files
        .iter()
        .map(|file| file.path.clone())
        .collect::<Vec<_>>();
    for impact in &report.impact_paths.files {
        evidence_paths.push(impact.path.clone());
        evidence_paths.extend(impact.direct_dependents.iter().cloned());
        evidence_paths.extend(impact.transitive_dependents.iter().cloned());
    }
    evidence_paths.sort();
    evidence_paths.dedup();

    let started = Instant::now();
    report.verification = run_checks_observed(
        &checks,
        &evidence_paths,
        session.revision(),
        cancellation,
        &mut |event| observer(map_event(event)),
    );
    report.timings.verification_us = started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
    Ok(())
}

fn map_event(event: VerificationExecutionEvent) -> ReviewVerificationEvent {
    match event {
        VerificationExecutionEvent::Started {
            check_id,
            index,
            total,
        } => ReviewVerificationEvent::Started {
            check_id,
            index,
            total,
        },
        VerificationExecutionEvent::Completed {
            check_id,
            index,
            total,
            status,
        } => ReviewVerificationEvent::Completed {
            check_id,
            index,
            total,
            status,
        },
    }
}

fn usage_error(error: impl std::fmt::Display) -> Box<dyn std::error::Error> {
    Box::new(CliExit {
        code: EXIT_USAGE,
        message: error.to_string(),
    })
}

#[cfg(test)]
mod tests;
