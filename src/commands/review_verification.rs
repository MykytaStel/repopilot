use crate::commands::{CliExit, EXIT_USAGE};
use repopilot::config::model::RepoPilotConfig;
use repopilot::review::diff::OwnedDiffTarget;
use repopilot::review::model::ReviewReport;
use repopilot::scan::session::AnalysisSession;
use repopilot::verification::{
    CancellationToken, run_checks, select_checks, validate_review_target,
};
use std::time::Instant;

pub(super) fn run_selected(
    selected: &[String],
    config: &RepoPilotConfig,
    session: &AnalysisSession,
    target: &OwnedDiffTarget,
    report: &mut ReviewReport,
) -> Result<(), Box<dyn std::error::Error>> {
    if selected.is_empty() {
        return Ok(());
    }
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
    report.verification = run_checks(
        &checks,
        &evidence_paths,
        session.revision(),
        &CancellationToken::new(),
    );
    report.timings.verification_us = started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
    Ok(())
}

fn usage_error(error: impl std::fmt::Display) -> Box<dyn std::error::Error> {
    Box::new(CliExit {
        code: EXIT_USAGE,
        message: error.to_string(),
    })
}
