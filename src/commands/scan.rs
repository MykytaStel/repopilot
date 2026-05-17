use crate::cli::ScanOptions;
use crate::commands::progress::{finish_spinner, make_spinner};
use crate::commands::{
    CliExit, EXIT_FINDINGS, EXIT_USAGE, ScanConfigOverrides, apply_min_priority_filter,
    apply_min_severity_filter, build_scan_config, finding_meets_min_priority,
    scan_options_min_priority, scan_options_min_severity,
};
use repopilot::baseline::diff::{
    BaselineScanReport, all_findings_new, diff_summary_against_baseline,
};
use repopilot::baseline::gate::{FailOn, evaluate_ci_gate};
use repopilot::baseline::reader::read_baseline;
use repopilot::config::loader::{load_default_config, load_optional_config};
use repopilot::config::presets::{Preset, apply_preset};
use repopilot::findings::visibility::{FindingVisibilityProfile, apply_visibility_profile};
use repopilot::output::{render_baseline_scan_report, render_scan_summary};
use repopilot::receipt::{build_audit_receipt, render_receipt_json};
use repopilot::report::writer::write_report;
use repopilot::risk::RiskPriority;
use repopilot::scan::scanner::scan_path_with_config;
use repopilot::scan::types::ScanSummary;
use repopilot::scan::workspace_scan::scan_workspace_with_config;
use std::path::Path;
use std::time::Instant;

pub fn run(options: ScanOptions) -> Result<(), Box<dyn std::error::Error>> {
    let min_severity = scan_options_min_severity(&options);
    let min_priority = scan_options_min_priority(&options);
    let fail_on_priority = options.fail_on_priority.map(Into::into);

    if options.fail_on.is_some() && fail_on_priority.is_some() {
        return Err(Box::new(CliExit {
            code: EXIT_USAGE,
            message: "`--fail-on` and `--fail-on-priority` cannot be used together".to_string(),
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

    let mut summary = if options.workspace {
        scan_workspace_with_config(&options.path, &scan_config)?
    } else {
        scan_path_with_config(&options.path, &scan_config)?
    };

    let scan_elapsed = scan_start.elapsed();
    finish_spinner(pb);

    if let Some(min) = min_severity {
        apply_min_severity_filter(&mut summary, min);
    }

    if !options.rule.is_empty() {
        apply_rule_filter(&mut summary, &options.rule);
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
        if let Some(min) = min_priority {
            apply_min_priority_filter_to_baseline_report(&mut baseline_report, min);
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

    if let Some(min) = min_priority {
        apply_min_priority_filter(&mut summary, min);
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

    Ok(())
}

fn apply_min_priority_filter_to_baseline_report(
    report: &mut BaselineScanReport,
    min: RiskPriority,
) {
    let mut paired = report
        .summary
        .findings
        .drain(..)
        .zip(report.findings.drain(..))
        .collect::<Vec<_>>();

    paired.retain(|(finding, _)| finding_meets_min_priority(finding, min));

    for (finding, status) in paired {
        report.summary.findings.push(finding);
        report.findings.push(status);
    }

    report.summary.visible_findings_count = report.summary.findings.len();
    report.summary.health_score =
        ScanSummary::compute_health_score(&report.summary.findings, report.summary.lines_of_code);
}

fn apply_rule_filter(summary: &mut ScanSummary, rules: &[String]) {
    summary
        .findings
        .retain(|f| rules.iter().any(|r| r == &f.rule_id));
    summary.visible_findings_count = summary.findings.len();
    summary.health_score =
        ScanSummary::compute_health_score(&summary.findings, summary.lines_of_code);
}

fn scan_visibility_profile(options: &ScanOptions) -> FindingVisibilityProfile {
    if options.include_maintainability || options.verbose || !options.rule.is_empty() {
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
