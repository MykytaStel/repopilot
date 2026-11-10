use super::{ReviewPlan, duration_us};
use crate::cli::ReviewOptions;
use crate::commands::product_scan::enforce_diagnostics_exit_policy;
use crate::commands::review_verification::run_selected;
use crate::commands::{CliExit, EXIT_FINDINGS};
use repopilot::baseline::gate::{CiGateResult, FailOn, evaluate_ci_gate};
use repopilot::config::model::ReviewScope;
use repopilot::history::{AnalysisScope, ReceiptContext, record_session};
use repopilot::output::{DetailLevel, FindingRenderLimit, OutputFormat};
use repopilot::report::writer::write_report;
use repopilot::review::diff::OwnedDiffTarget;
use repopilot::review::model::ReviewReport;
use repopilot::review::render::{
    ReviewRenderOptions, render_review_sarif_with_gates, render_with_options,
};
use repopilot::review::{ReviewSignalGateResult, review_report_for_ci};
use repopilot::scan::session::AnalysisSession;
use repopilot::verification::VerificationStatus;
use std::error::Error;
use std::time::Instant;

pub(super) fn complete(
    options: &ReviewOptions,
    plan: &ReviewPlan,
    session: &AnalysisSession,
    report: &mut ReviewReport,
) -> Result<(), Box<dyn Error>> {
    run_selected(&options.verify, session, &plan.history_target, report)?;
    let gates = evaluate_gates(options, plan, report);
    record_history(options, plan, session, report);
    render_report(options, &gates, report)?;
    enforce_gates(&gates, report)?;
    Ok(())
}

struct ReviewGates {
    ci: Option<CiGateResult>,
    review: ReviewSignalGateResult,
}

fn evaluate_gates(
    options: &ReviewOptions,
    plan: &ReviewPlan,
    report: &mut ReviewReport,
) -> ReviewGates {
    let started = Instant::now();
    let ci_report = review_report_for_ci(report);
    let ci = options
        .fail_on
        .map(Into::into)
        .map(|fail_on| evaluate_ci_gate(&ci_report, fail_on))
        .or_else(|| {
            options
                .fail_on_priority
                .map(Into::into)
                .map(|priority| evaluate_ci_gate(&ci_report, FailOn::Priority(priority)))
        });
    let review = ReviewSignalGateResult::evaluate(report, plan.review_gate_policy);
    report.timings.gating_us = duration_us(started.elapsed());
    ReviewGates { ci, review }
}

fn record_history(
    options: &ReviewOptions,
    plan: &ReviewPlan,
    session: &AnalysisSession,
    report: &mut ReviewReport,
) {
    if !(options.record_history || session.repo_config().history.enabled)
        || report.summary.has_error_diagnostics()
    {
        return;
    }
    let outcome = record_session(
        session,
        review_history_context(plan.scope, &plan.history_target, options),
        &report.summary.artifacts.findings,
    );
    if let Some(repopilot::history::ComparisonResult::Compatible(delta)) = outcome.comparison {
        report.summary.artifacts.risk_delta = Some(delta);
    }
    for diagnostic in outcome.diagnostics {
        report
            .summary
            .artifacts
            .diagnostics
            .push(repopilot::scan::types::ScanDiagnostic::warning(
                format!("history.{}", diagnostic.kind.code()),
                diagnostic.message,
            ));
    }
}

fn render_report(
    options: &ReviewOptions,
    gates: &ReviewGates,
    report: &mut ReviewReport,
) -> Result<(), Box<dyn Error>> {
    let output_format: OutputFormat = options.format.into();
    let render_options = ReviewRenderOptions {
        detail: options
            .detail
            .map(Into::into)
            .unwrap_or(DetailLevel::Findings),
        findings_limit: options
            .max_findings
            .map(Into::into)
            .unwrap_or(FindingRenderLimit::Default),
    };
    let started = Instant::now();
    let mut rendered = render_with_options(
        report,
        output_format,
        gates.ci.as_ref(),
        Some(&gates.review),
        render_options,
    )?;
    report.timings.rendering_us = duration_us(started.elapsed());
    if output_format == OutputFormat::Json {
        rendered = render_with_options(
            report,
            output_format,
            gates.ci.as_ref(),
            Some(&gates.review),
            render_options,
        )?;
    }
    write_report(&rendered, options.output.as_deref())?;
    write_sarif(options, gates, report)?;
    enforce_diagnostics_exit_policy(&report.summary)?;
    Ok(())
}

fn write_sarif(
    options: &ReviewOptions,
    gates: &ReviewGates,
    report: &ReviewReport,
) -> Result<(), Box<dyn Error>> {
    let Some(path) = options.sarif_output.as_deref() else {
        return Ok(());
    };
    let sarif = render_review_sarif_with_gates(report, gates.ci.as_ref(), Some(&gates.review))?;
    write_report(&sarif, Some(path))?;
    Ok(())
}

fn enforce_gates(gates: &ReviewGates, report: &ReviewReport) -> Result<(), CliExit> {
    if let Some(gate) = &gates.ci
        && let Some(message) = gate.failure_message()
    {
        return Err(CliExit {
            code: EXIT_FINDINGS,
            message,
        });
    }
    if !gates.review.passed() {
        return Err(CliExit {
            code: EXIT_FINDINGS,
            message: format!(
                "RepoPilot review gate failed: {} definitely-sensitive signal(s)",
                gates.review.failed_signals
            ),
        });
    }
    if report.verification.iter().any(|outcome| {
        !outcome.revision_compatible
            || matches!(
                outcome.status,
                VerificationStatus::Failed
                    | VerificationStatus::TimedOut
                    | VerificationStatus::Unavailable
                    | VerificationStatus::Cancelled
            )
    }) {
        return Err(CliExit {
            code: EXIT_FINDINGS,
            message: "RepoPilot verification blocked merge readiness".to_string(),
        });
    }
    Ok(())
}

fn review_history_context(
    scope: ReviewScope,
    target: &OwnedDiffTarget,
    options: &ReviewOptions,
) -> ReceiptContext {
    let analysis_scope = match scope {
        ReviewScope::Changed => AnalysisScope::ReviewChanged,
        ReviewScope::Full => AnalysisScope::ReviewFull,
    };
    let selection_fingerprint = repopilot::scan::cache::stable_hash_hex(
        format!(
            "{:?}|{:?}|{:?}|{:?}|{:?}",
            options.scope,
            options.min_severity,
            options.min_confidence,
            options.min_priority,
            options.profile
        )
        .as_bytes(),
    );
    let (base_ref, head_ref) = match target {
        OwnedDiffTarget::WorkingTree => (Some("HEAD".to_string()), None),
        OwnedDiffTarget::Refs { base, head } => (Some(base.clone()), Some(head.clone())),
        OwnedDiffTarget::SinceRef { base } => (Some(base.clone()), None),
    };
    ReceiptContext {
        scope: analysis_scope,
        base_ref,
        head_ref,
        selection_fingerprint,
    }
}
