use crate::cli::ReviewOptions;
use crate::commands::progress::{finish_spinner, make_spinner};
use crate::commands::{
    CliExit, EXIT_FINDINGS, EXIT_USAGE, ScanConfigOverrides, apply_min_severity_filter,
    build_scan_config, finding_meets_min_priority, review_options_min_priority,
    review_options_min_severity,
};
use repopilot::baseline::gate::{FailOn, evaluate_ci_gate};
use repopilot::baseline::reader::read_baseline;
use repopilot::config::loader::{load_default_config, load_optional_config};
use repopilot::report::writer::write_report;
use repopilot::review::model::ReviewReport;
use repopilot::review::render::render;
use repopilot::review::{build_review_report, review_report_for_ci};
use repopilot::risk::RiskPriority;
use repopilot::scan::scanner::scan_path_with_config;

pub fn run(options: ReviewOptions) -> Result<(), Box<dyn std::error::Error>> {
    let min_severity = review_options_min_severity(&options);
    let min_priority = review_options_min_priority(&options);
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

    let repo_config = match &options.config {
        Some(config_path) => load_optional_config(config_path)?,
        None => load_default_config()?,
    };
    let scan_config = build_scan_config(
        &repo_config,
        ScanConfigOverrides {
            max_file_loc: options.max_file_loc,
            max_directory_modules: options.max_directory_modules,
            max_directory_depth: options.max_directory_depth,
            ..ScanConfigOverrides::default()
        },
    );

    let pb = make_spinner("Scanning...");
    let mut summary = scan_path_with_config(&options.path, &scan_config)?;
    finish_spinner(pb);

    if let Some(min) = min_severity {
        apply_min_severity_filter(&mut summary, min);
    }

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
    )?;
    if let Some(min) = min_priority {
        apply_min_priority_filter_to_review_report(&mut review_report, min);
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

fn apply_min_priority_filter_to_review_report(report: &mut ReviewReport, min: RiskPriority) {
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

    report.summary.health_score = repopilot::scan::types::ScanSummary::compute_health_score(
        &report.summary.findings,
        report.summary.lines_of_code,
    );
}
