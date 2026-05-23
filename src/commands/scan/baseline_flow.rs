use crate::cli::ScanOptions;
use crate::commands::product_scan::enforce_diagnostics_exit_policy;
use crate::commands::{CliExit, EXIT_FINDINGS};
use repopilot::baseline::diff::{all_findings_new, diff_summary_against_baseline};
use repopilot::baseline::gate::{FailOn, evaluate_ci_gate};
use repopilot::baseline::reader::read_baseline;
use repopilot::findings::filter::FindingFilter;
use repopilot::output::{OutputFormat, render_baseline_scan_report};
use repopilot::report::writer::write_report;
use repopilot::risk::RiskPriority;
use repopilot::scan::types::ScanSummary;
use std::time::{Duration, Instant};

pub(super) fn uses_baseline_flow(options: &ScanOptions) -> bool {
    options.baseline.is_some() || options.fail_on.is_some() || options.fail_on_priority.is_some()
}

pub(super) fn run_baseline_scan_flow(
    summary: ScanSummary,
    options: &ScanOptions,
    priority_filter: FindingFilter,
    fail_on_priority: Option<RiskPriority>,
    output_format: OutputFormat,
    scan_elapsed: Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut baseline_report = match options.baseline.clone() {
        Some(baseline_path) => {
            let baseline_file = read_baseline(&baseline_path)?;
            diff_summary_against_baseline(summary, &baseline_file, baseline_path)
        }
        None => all_findings_new(summary),
    };

    if !priority_filter.is_empty() {
        baseline_report.apply_filter(&priority_filter);
    }

    let ci_gate = options
        .fail_on
        .map(Into::into)
        .map(|fail_on| evaluate_ci_gate(&baseline_report, fail_on))
        .or_else(|| {
            fail_on_priority
                .map(|priority| evaluate_ci_gate(&baseline_report, FailOn::Priority(priority)))
        });

    let render_start = Instant::now();
    let rendered_report =
        render_baseline_scan_report(&baseline_report, output_format, ci_gate.as_ref())?;
    let render_elapsed = render_start.elapsed();

    super::write_scan_receipt_if_requested(&baseline_report.summary, options.receipt.as_deref())?;
    write_report(&rendered_report, options.output.as_deref())?;

    if options.verbose {
        super::timing::print_verbose_scan_timing(
            &baseline_report.summary,
            scan_elapsed,
            render_elapsed,
        );
    }

    if options.timing {
        super::timing::print_timing_breakdown(&baseline_report.summary);
    }

    enforce_diagnostics_exit_policy(&baseline_report.summary)?;

    if let Some(ci_gate) = ci_gate
        && let Some(message) = ci_gate.failure_message()
    {
        return Err(Box::new(CliExit {
            code: EXIT_FINDINGS,
            message,
        }));
    }

    Ok(())
}
