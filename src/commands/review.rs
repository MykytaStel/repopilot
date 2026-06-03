use crate::cli::ReviewOptions;
use crate::commands::filters::{review_pre_diff_filter, review_priority_filter};
use crate::commands::product_scan::{
    ProductScanMode, ProductScanRequest, enforce_diagnostics_exit_policy, run_product_scan,
};
use crate::commands::scan_config::ScanConfigOverrides;
use crate::commands::{CliExit, EXIT_FINDINGS, EXIT_USAGE};
use repopilot::baseline::gate::{FailOn, evaluate_ci_gate};
use repopilot::baseline::reader::read_baseline;
use repopilot::findings::visibility::FindingVisibilityProfile;
use repopilot::report::writer::write_report;
use repopilot::review::render::render;
use repopilot::review::{build_review_report, review_report_for_ci};

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
        mode: ProductScanMode::Full,
        no_progress: options.no_progress,
        ignore_feedback: options.ignore_feedback,
        visibility_profile: FindingVisibilityProfile::Strict,
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
    let mut review_report = build_review_report(
        summary,
        &options.path,
        options.base.as_deref(),
        options.head.as_deref(),
        baseline_ref,
        &scan_result.repo_config,
    )?;
    if !priority_filter.is_empty() {
        review_report.apply_filter(&priority_filter);
    }

    let ci_report = review_report_for_ci(&review_report);
    let ci_gate = options
        .fail_on
        .map(Into::into)
        .map(|fail_on| evaluate_ci_gate(&ci_report, fail_on))
        .or_else(|| {
            fail_on_priority
                .map(|priority| evaluate_ci_gate(&ci_report, FailOn::Priority(priority)))
        });
    let rendered_report = render(&review_report, options.format.into(), ci_gate.as_ref())?;

    write_report(&rendered_report, options.output.as_deref())?;
    enforce_diagnostics_exit_policy(&review_report.summary)?;

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
