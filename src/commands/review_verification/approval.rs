use super::{ReviewVerificationEvent, VerificationApproval, usage_error};
use repopilot::review::diff::OwnedDiffTarget;
use repopilot::review::model::ReviewReport;
use repopilot::scan::session::{AnalysisSession, WorkspaceRevision};
use repopilot::verification::{
    CancellationToken, ValidatedCheck, VerificationExecutionEvent, VerificationOutcome,
    VerificationStatus, run_checks_observed_cached, select_checks, skipped_outcome,
    validate_review_target,
};
use std::time::Instant;

#[cfg(test)]
#[path = "approval_tests.rs"]
mod tests;

pub(crate) fn run_selected_with_approval(
    selected: &[String],
    session: &AnalysisSession,
    target: &OwnedDiffTarget,
    report: &mut ReviewReport,
    cancellation: &CancellationToken,
    observer: &mut dyn FnMut(ReviewVerificationEvent),
    reload_check: &mut dyn FnMut(&str) -> Result<ValidatedCheck, String>,
    approve: &mut dyn FnMut(&ValidatedCheck) -> VerificationApproval,
) -> Result<(), Box<dyn std::error::Error>> {
    report.verification_policy.set_selected(selected);
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
    let evidence_paths = evidence_paths(report);
    let started = Instant::now();
    let total = checks.len();

    for (offset, check) in checks.iter().enumerate() {
        let index = offset + 1;
        if cancellation.is_cancelled() {
            break;
        }
        observer(ReviewVerificationEvent::Started {
            check_id: check.id().to_string(),
            index,
            total,
        });
        match approve(check) {
            VerificationApproval::ToolCallCancelled => break,
            VerificationApproval::Skip {
                limitation,
                stop_following,
            } => {
                report.verification.push(skipped_outcome(
                    check,
                    session.revision(),
                    &limitation,
                    true,
                ));
                observer(ReviewVerificationEvent::Completed {
                    check_id: check.id().to_string(),
                    index,
                    total,
                    status: VerificationStatus::Skipped,
                });
                if stop_following {
                    for (later_offset, later) in checks.iter().enumerate().skip(offset + 1) {
                        append_skipped(
                            later,
                            later_offset + 1,
                            total,
                            session.revision(),
                            "approval stopped after previous check",
                            observer,
                            &mut report.verification,
                        );
                    }
                    break;
                }
                continue;
            }
            VerificationApproval::Accepted => {}
        }

        let current_revision = WorkspaceRevision::capture(session.workspace_root());
        if &current_revision != session.revision() {
            report.verification.push(skipped_outcome(
                check,
                session.revision(),
                "workspace changed before execution",
                false,
            ));
            notify_completed(check, index, total, VerificationStatus::Skipped, observer);
            continue;
        }

        let current_check = reload_check(check.id());
        if !matches!(current_check, Ok(ref current) if check.same_execution_policy(current)) {
            report.verification.push(skipped_outcome(
                check,
                session.revision(),
                "check definition changed before execution",
                true,
            ));
            notify_completed(check, index, total, VerificationStatus::Skipped, observer);
            continue;
        }

        let outcomes = run_checks_observed_cached(
            std::slice::from_ref(check),
            &evidence_paths,
            session.revision(),
            cancellation,
            Some(session.workspace_root()),
            &mut |event| {
                if let Some(event) = map_event(event, index, total) {
                    observer(event);
                }
            },
        );
        report.verification.extend(outcomes);
    }

    report.timings.verification_us = started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
    Ok(())
}

pub(crate) fn evidence_paths(report: &ReviewReport) -> Vec<std::path::PathBuf> {
    let mut paths = report
        .changed_files
        .iter()
        .map(|file| file.path.clone())
        .collect::<Vec<_>>();
    for impact in &report.impact_paths.files {
        paths.push(impact.path.clone());
        paths.extend(impact.direct_dependents.iter().cloned());
        paths.extend(impact.transitive_dependents.iter().cloned());
    }
    paths.sort();
    paths.dedup();
    paths
}

fn append_skipped(
    check: &ValidatedCheck,
    index: usize,
    total: usize,
    revision: &WorkspaceRevision,
    limitation: &str,
    observer: &mut dyn FnMut(ReviewVerificationEvent),
    outcomes: &mut Vec<VerificationOutcome>,
) {
    observer(ReviewVerificationEvent::Started {
        check_id: check.id().to_string(),
        index,
        total,
    });
    outcomes.push(skipped_outcome(check, revision, limitation, true));
    notify_completed(check, index, total, VerificationStatus::Skipped, observer);
}

fn notify_completed(
    check: &ValidatedCheck,
    index: usize,
    total: usize,
    status: VerificationStatus,
    observer: &mut dyn FnMut(ReviewVerificationEvent),
) {
    observer(ReviewVerificationEvent::Completed {
        check_id: check.id().to_string(),
        index,
        total,
        status,
    });
}

fn map_event(
    event: VerificationExecutionEvent,
    index: usize,
    total: usize,
) -> Option<ReviewVerificationEvent> {
    match event {
        VerificationExecutionEvent::Started { .. } => None,
        VerificationExecutionEvent::Completed {
            check_id, status, ..
        } => Some(ReviewVerificationEvent::Completed {
            check_id,
            index,
            total,
            status,
        }),
    }
}
