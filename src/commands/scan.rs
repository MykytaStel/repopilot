mod baseline_flow;
mod timing;
mod validation;

use crate::cli::ScanOptions;
use crate::commands::filters::{scan_pre_visibility_filter, scan_priority_filter};
use crate::commands::product_scan::{
    ProductScanRequest, enforce_diagnostics_exit_policy, run_product_scan,
};
use crate::commands::scan_config::ScanConfigOverrides;
use repopilot::output::render_scan_summary;
use repopilot::receipt::{build_audit_receipt, render_receipt_json};
use repopilot::report::writer::write_report;
use repopilot::scan::types::ScanSummary;
use std::path::Path;
use std::time::Instant;

pub fn run(options: ScanOptions) -> Result<(), Box<dyn std::error::Error>> {
    validation::validate_scan_options(&options)?;

    let pre_visibility_filter = scan_pre_visibility_filter(&options);
    let priority_filter = scan_priority_filter(&options);
    let fail_on_priority = options.fail_on_priority.map(Into::into);
    let visibility_profile = validation::scan_visibility_profile(&options);
    let scan_mode = validation::scan_mode_from_options(&options);

    let scan_result = run_product_scan(ProductScanRequest {
        path: options.path.clone(),
        config_path: options.config.clone(),
        overrides: ScanConfigOverrides {
            max_file_loc: options.max_file_loc,
            max_directory_modules: options.max_directory_modules,
            max_directory_depth: options.max_directory_depth,
            exclude_patterns: options.exclude.clone(),
            include_low_signal: options.include_low_signal,
            max_file_size: options.max_file_size,
            max_files: options.max_files,
        },
        preset: options.preset.clone(),
        mode: scan_mode,
        ignore_feedback: options.ignore_feedback,
        visibility_profile,
        pre_visibility_filter,
    })?;

    let mut summary = scan_result.summary;
    let scan_elapsed = scan_result.scan_elapsed;
    let output_format = options
        .format
        .map(Into::into)
        .unwrap_or(scan_result.repo_config.output.default_format);

    if baseline_flow::uses_baseline_flow(&options) {
        return baseline_flow::run_baseline_scan_flow(
            summary,
            &options,
            priority_filter,
            fail_on_priority,
            output_format,
            scan_elapsed,
        );
    }

    if !priority_filter.is_empty() {
        priority_filter.apply_to_summary(&mut summary);
    }

    let render_start = Instant::now();
    let rendered_report = render_scan_summary(&summary, output_format)?;
    let render_elapsed = render_start.elapsed();

    write_scan_receipt_if_requested(&summary, options.receipt.as_deref())?;
    write_report(&rendered_report, options.output.as_deref())?;

    if options.verbose {
        timing::print_verbose_scan_timing(&summary, scan_elapsed, render_elapsed);
    }

    if options.timing {
        timing::print_timing_breakdown(&summary);
    }

    enforce_diagnostics_exit_policy(&summary)?;

    Ok(())
}

pub(super) fn write_scan_receipt_if_requested(
    summary: &ScanSummary,
    receipt_path: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(receipt_path) = receipt_path else {
        return Ok(());
    };

    let receipt = build_audit_receipt(summary);
    let rendered = render_receipt_json(&receipt)?;

    write_report(&rendered, Some(receipt_path))?;

    Ok(())
}
