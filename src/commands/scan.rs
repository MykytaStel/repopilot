use crate::cli::ScanOptions;
use crate::commands::filters::{scan_pre_visibility_filter, scan_priority_filter};
use crate::commands::progress::{finish_spinner, make_spinner};
use crate::commands::scan_config::{ScanConfigOverrides, build_scan_config};
use crate::commands::{CliExit, EXIT_FINDINGS, EXIT_RUNTIME, EXIT_USAGE};
use repopilot::baseline::diff::{all_findings_new, diff_summary_against_baseline};
use repopilot::baseline::gate::{FailOn, evaluate_ci_gate};
use repopilot::baseline::reader::read_baseline;
use repopilot::config::loader::{load_default_config, load_optional_config};
use repopilot::config::presets::{Preset, apply_preset};
use repopilot::findings::feedback::apply_local_feedback;
use repopilot::findings::visibility::{FindingVisibilityProfile, apply_visibility_profile};
use repopilot::output::{render_baseline_scan_report, render_scan_summary};
use repopilot::receipt::{build_audit_receipt, render_receipt_json};
use repopilot::report::writer::write_report;
use repopilot::scan::scanner::{scan_changed_with_config, scan_path_with_config};
use repopilot::scan::types::ScanSummary;
use repopilot::scan::workspace_scan::scan_workspace_with_config;
use std::path::Path;
use std::time::Instant;

pub fn run(options: ScanOptions) -> Result<(), Box<dyn std::error::Error>> {
    let pre_visibility_filter = scan_pre_visibility_filter(&options);
    let priority_filter = scan_priority_filter(&options);
    let fail_on_priority = options.fail_on_priority.map(Into::into);

    if options.fail_on.is_some() && fail_on_priority.is_some() {
        return Err(Box::new(CliExit {
            code: EXIT_USAGE,
            message: "`--fail-on` and `--fail-on-priority` cannot be used together".to_string(),
        }));
    }
    if options.changed && options.since.is_some() {
        return Err(Box::new(CliExit {
            code: EXIT_USAGE,
            message: "`--changed` and `--since` cannot be used together".to_string(),
        }));
    }
    if options.workspace && (options.changed || options.since.is_some()) {
        return Err(Box::new(CliExit {
            code: EXIT_USAGE,
            message: "`--workspace` cannot be used with changed scans".to_string(),
        }));
    }
    let mut repo_config = match &options.config {
        Some(config_path) => load_optional_config(config_path)?,
        None => load_default_config()?,
    };

    if let Some(preset_str) = options.preset.as_deref() {
        match preset_str.parse::<Preset>() {
            Ok(p) => apply_preset(&mut repo_config, p),
            Err(_) => {
                return Err(Box::new(CliExit {
                    code: EXIT_USAGE,
                    message: format!(
                        "Invalid preset '{preset_str}'. Expected: strict, balanced, lenient"
                    ),
                }));
            }
        }
    }

    let scan_config = build_scan_config(
        &repo_config,
        ScanConfigOverrides {
            max_file_loc: options.max_file_loc,
            max_directory_modules: options.max_directory_modules,
            max_directory_depth: options.max_directory_depth,
            exclude_patterns: options.exclude.clone(),
            include_low_signal: options.include_low_signal,
            max_file_size: options.max_file_size,
            max_files: options.max_files,
        },
    );
    let output_format = options
        .format
        .map(Into::into)
        .unwrap_or(repo_config.output.default_format);

    let pb = make_spinner("Scanning...");
    let scan_start = Instant::now();

    let mut summary = if options.changed || options.since.is_some() {
        scan_changed_with_config(&options.path, &scan_config, options.since.as_deref())?
    } else if options.workspace {
        scan_workspace_with_config(&options.path, &scan_config)?
    } else {
        scan_path_with_config(&options.path, &scan_config)?
    };

    let scan_elapsed = scan_start.elapsed();
    finish_spinner(pb);

    if !options.ignore_feedback {
        apply_local_feedback(&mut summary, &options.path)?;
    }

    if !pre_visibility_filter.is_empty() {
        pre_visibility_filter.apply_to_summary(&mut summary);
    }

    let visibility_profile = scan_visibility_profile(&options);
    apply_visibility_profile(&mut summary, visibility_profile);

    if options.baseline.is_some() || options.fail_on.is_some() || fail_on_priority.is_some() {
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

        write_scan_receipt_if_requested(&baseline_report.summary, options.receipt.as_deref())?;
        write_report(&rendered_report, options.output.as_deref())?;

        if options.verbose {
            let internal_us = baseline_report.summary.scan_duration_us;
            let total_ms = scan_elapsed.as_millis();
            let render_ms = render_elapsed.as_millis();
            eprintln!(
                "\n[verbose] Scan: {total_ms}ms (engine: {}ms) · Render: {render_ms}ms",
                internal_us / 1000
            );
        }

        if options.timing {
            print_timing_breakdown(&baseline_report.summary);
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

        return Ok(());
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
        let internal_us = summary.scan_duration_us;
        let total_ms = scan_elapsed.as_millis();
        let render_ms = render_elapsed.as_millis();
        eprintln!(
            "\n[verbose] Scan: {total_ms}ms (engine: {:.0}ms) · Render: {render_ms}ms",
            internal_us as f64 / 1000.0
        );
    }

    if options.timing {
        print_timing_breakdown(&summary);
    }

    enforce_diagnostics_exit_policy(&summary)?;

    Ok(())
}

fn scan_visibility_profile(options: &ScanOptions) -> FindingVisibilityProfile {
    if options.include_maintainability || !options.rule.is_empty() {
        return FindingVisibilityProfile::Strict;
    }

    match options.profile {
        Some(crate::cli::ScanProfileArg::Strict) => FindingVisibilityProfile::Strict,
        Some(crate::cli::ScanProfileArg::Default) | None => FindingVisibilityProfile::Default,
    }
}

fn print_timing_breakdown(summary: &ScanSummary) {
    if let Some(timings) = &summary.scan_timings {
        let total =
            timings.file_scan_us + timings.framework_detection_us + timings.post_scan_audits_us;
        eprintln!(
            "\n[timing] File scan: {}ms · Framework detection: {}ms · Post-scan audits: {}ms · Engine total: {}ms",
            timings.file_scan_us / 1000,
            timings.framework_detection_us / 1000,
            timings.post_scan_audits_us / 1000,
            total / 1000,
        );
    }

    if let Some(cache) = &summary.cache_telemetry {
        let estimated_saved = cache
            .timings
            .estimated_time_saved_us
            .map(|value| format!("{}ms", value / 1000))
            .unwrap_or_else(|| "n/a".to_string());
        eprintln!(
            "[timing] Cache: load {}ms · hash {}ms · lookup {}ms · hit reuse {}ms · miss scan {}ms · write {}ms · estimated saved {}",
            cache.timings.load_us / 1000,
            cache.timings.file_hash_us / 1000,
            cache.timings.lookup_us / 1000,
            cache.timings.hit_reuse_us / 1000,
            cache.timings.miss_scan_us / 1000,
            cache.timings.write_us / 1000,
            estimated_saved,
        );
    }
}

fn write_scan_receipt_if_requested(
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

fn enforce_diagnostics_exit_policy(
    summary: &ScanSummary,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(diagnostic) = summary.first_error_diagnostic() else {
        return Ok(());
    };

    Err(Box::new(CliExit {
        code: EXIT_RUNTIME,
        message: format!(
            "RepoPilot scan completed with reportable error diagnostic `{}`: {}",
            diagnostic.code, diagnostic.message
        ),
    }))
}
