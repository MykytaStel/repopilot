use crate::cli::ReviewOptions;
use crate::cli::{ReviewFailOnArg, ReviewScopeArg, ScanProfileArg};
use crate::commands::filters::{review_pre_diff_filter, review_priority_filter};
use crate::commands::product_scan::{
    ProductScanMode, ProductScanRequest, enforce_diagnostics_exit_policy, run_product_scan,
};
use crate::commands::scan_config::ScanConfigOverrides;
use crate::commands::{CliExit, EXIT_FINDINGS, EXIT_USAGE};
use repopilot::baseline::gate::{FailOn, evaluate_ci_gate};
use repopilot::baseline::reader::read_baseline;
use repopilot::config::loader::{load_default_config, load_optional_config};
use repopilot::config::model::{ReviewFailOn, ReviewScope};
use repopilot::findings::visibility::FindingVisibilityProfile;
use repopilot::output::OutputFormat;
use repopilot::report::writer::write_report;
use repopilot::review::render::{render, render_review_sarif};
use repopilot::review::{
    ReviewSignalGatePolicy, ReviewSignalGateResult, build_review_report_from_session,
    load_review_input, load_review_input_since, review_report_for_ci,
};
use std::time::{Duration, Instant};

pub fn run(options: ReviewOptions) -> Result<(), Box<dyn std::error::Error>> {
    let pre_diff_filter = review_pre_diff_filter(&options);
    let priority_filter = review_priority_filter(&options);
    let fail_on_priority = options.fail_on_priority.map(Into::into);

    if options.fail_on.is_some() && fail_on_priority.is_some() {
        return Err(Box::new(CliExit {
            code: EXIT_USAGE,
            message: "`--fail-on` and `--fail-on-priority` cannot be used together".to_string(),
        }));
    }

    if options.base.is_none() && options.head.is_some() {
        return Err(Box::new(CliExit {
            code: EXIT_USAGE,
            message: "`repopilot review --head` requires --base".to_string(),
        }));
    }

    if options.since_snapshot && (options.base.is_some() || options.head.is_some()) {
        return Err(Box::new(CliExit {
            code: EXIT_USAGE,
            message: "`--since-snapshot` cannot be used with --base or --head".to_string(),
        }));
    }

    let configured = match &options.config {
        Some(path) => load_optional_config(path)?,
        None => load_default_config()?,
    };
    let scope = match options.scope {
        Some(ReviewScopeArg::Changed) => ReviewScope::Changed,
        Some(ReviewScopeArg::Full) => ReviewScope::Full,
        None => configured.review.scope,
    };
    let visibility_profile = match options.profile {
        Some(ScanProfileArg::Default) => FindingVisibilityProfile::Default,
        Some(ScanProfileArg::Strict) => FindingVisibilityProfile::Strict,
        None if scope == ReviewScope::Full => FindingVisibilityProfile::Strict,
        None => FindingVisibilityProfile::Default,
    };
    let review_gate_policy = match options.fail_on_review {
        Some(ReviewFailOnArg::None) => ReviewSignalGatePolicy::None,
        Some(ReviewFailOnArg::Definitely) => ReviewSignalGatePolicy::Definitely,
        None => match configured.review.fail_on {
            ReviewFailOn::None => ReviewSignalGatePolicy::None,
            ReviewFailOn::Definitely => ReviewSignalGatePolicy::Definitely,
        },
    };
    let diff_started = Instant::now();
    let review_input = if options.since_snapshot {
        let snapshot = crate::commands::snapshot::read_snapshot(&options.path)?;
        load_review_input_since(&options.path, &snapshot.head)?
    } else {
        load_review_input(
            &options.path,
            options.base.as_deref(),
            options.head.as_deref(),
        )?
    };
    let diff_loading_us = duration_us(diff_started.elapsed());
    let scan_mode = match scope {
        ReviewScope::Changed => ProductScanMode::ResolvedChanged {
            changed_files: review_input.changed_files.clone(),
            base_ref: review_input.target.base_ref().map(str::to_string),
        },
        ReviewScope::Full => ProductScanMode::Full,
    };

    let scan_result = run_product_scan(ProductScanRequest {
        path: options.path.clone(),
        config_path: options.config.clone(),
        overrides: ScanConfigOverrides {
            max_file_loc: options.max_file_loc,
            max_directory_modules: options.max_directory_modules,
            max_directory_depth: options.max_directory_depth,
            ..ScanConfigOverrides::default()
        },
        preset: None,
        mode: scan_mode,
        no_progress: options.no_progress,
        ignore_feedback: options.ignore_feedback,
        visibility_profile,
        pre_visibility_filter: pre_diff_filter,
    })?;
    let summary = scan_result.summary;

    let baseline_file = match options.baseline {
        Some(baseline_path) => Some((read_baseline(&baseline_path)?, baseline_path)),
        None => None,
    };
    let baseline_ref = baseline_file
        .as_ref()
        .map(|(baseline, path)| (baseline, path.clone()));
    let review_started = Instant::now();
    let mut review_report = build_review_report_from_session(
        summary,
        review_input,
        baseline_ref,
        &scan_result.session,
    )?;
    review_report.timings.diff_loading_us = diff_loading_us;
    review_report.timings.review_signals_us = duration_us(review_started.elapsed());
    if scope == ReviewScope::Changed {
        review_report.retain_in_diff_findings();
    }
    if !priority_filter.is_empty() {
        review_report.apply_filter(&priority_filter);
    }

    let gating_started = Instant::now();
    let ci_report = review_report_for_ci(&review_report);
    let ci_gate = options
        .fail_on
        .map(Into::into)
        .map(|fail_on| evaluate_ci_gate(&ci_report, fail_on))
        .or_else(|| {
            fail_on_priority
                .map(|priority| evaluate_ci_gate(&ci_report, FailOn::Priority(priority)))
        });
    let review_gate = ReviewSignalGateResult::evaluate(&review_report, review_gate_policy);
    review_report.timings.gating_us = duration_us(gating_started.elapsed());
    let output_format: OutputFormat = options.format.into();
    let rendering_started = Instant::now();
    let mut rendered_report = render(
        &review_report,
        output_format,
        ci_gate.as_ref(),
        Some(&review_gate),
    )?;
    review_report.timings.rendering_us = duration_us(rendering_started.elapsed());
    if output_format == OutputFormat::Json {
        rendered_report = render(
            &review_report,
            output_format,
            ci_gate.as_ref(),
            Some(&review_gate),
        )?;
    }

    write_report(&rendered_report, options.output.as_deref())?;
    if let Some(path) = options.sarif_output.as_deref() {
        write_report(&render_review_sarif(&review_report)?, Some(path))?;
    }
    enforce_diagnostics_exit_policy(&review_report.summary)?;

    if let Some(ci_gate) = ci_gate
        && let Some(message) = ci_gate.failure_message()
    {
        return Err(Box::new(CliExit {
            code: EXIT_FINDINGS,
            message,
        }));
    }
    if !review_gate.passed() {
        return Err(Box::new(CliExit {
            code: EXIT_FINDINGS,
            message: format!(
                "RepoPilot review gate failed: {} definitely-sensitive signal(s)",
                review_gate.failed_signals
            ),
        }));
    }

    Ok(())
}

fn duration_us(duration: Duration) -> u64 {
    duration.as_micros().min(u128::from(u64::MAX)) as u64
}
