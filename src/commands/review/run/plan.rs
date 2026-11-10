use super::ReviewPlan;
use crate::cli::{ReviewFailOnArg, ReviewOptions, ReviewScopeArg, ScanProfileArg};
use crate::commands::filters::{review_pre_diff_filter, review_priority_filter};
use crate::commands::product_scan::{ProductScanMode, ProductScanRequest, run_product_scan};
use crate::commands::review::run::duration_us;
use crate::commands::scan_config::ScanConfigOverrides;
use crate::commands::{CliExit, EXIT_USAGE};
use repopilot::config::loader::{ConfigError, load_default_config, load_optional_config};
use repopilot::config::model::{RepoPilotConfig, ReviewFailOn, ReviewScope};
use repopilot::findings::visibility::FindingVisibilityProfile;
use repopilot::output::OutputFormat;
use repopilot::report::writer::write_report;
use repopilot::review::intent::{IntentContext, load_intent_file, validate_critical_paths};
use repopilot::review::model::ReviewReport;
use repopilot::review::render::{ReviewRenderOptions, render_with_options};
use repopilot::review::{
    ReviewSignalGatePolicy, ReviewSignalGateResult, build_review_report_from_session,
    load_review_input, load_review_input_since,
};
use repopilot::scan::session::AnalysisSession;
use repopilot::scan::types::ScanSummary;
use std::error::Error;
use std::fmt;
use std::time::Instant;

pub(super) fn prepare(options: &ReviewOptions) -> Result<ReviewPlan, Box<dyn Error>> {
    validate_options(options)?;
    let config = load_config(options)?;
    validate_critical_paths(&config)?;
    let scope = review_scope(options, &config);
    let visibility_profile = visibility_profile(options, scope);
    let review_gate_policy = review_gate_policy(options, &config);
    let (review_input, diff_loading_us) = load_input(options)?;
    let intent = load_intent(options, &review_input, &config)?;
    Ok(ReviewPlan {
        priority_filter: review_priority_filter(options),
        scope,
        visibility_profile,
        review_gate_policy,
        history_target: review_input.target.clone(),
        review_input,
        intent,
        diff_loading_us,
    })
}

fn validate_options(options: &ReviewOptions) -> Result<(), CliExit> {
    if options.fail_on.is_some() && options.fail_on_priority.is_some() {
        return Err(usage_error(
            "`--fail-on` and `--fail-on-priority` cannot be used together",
        ));
    }
    if options.base.is_none() && options.head.is_some() {
        return Err(usage_error(
            "`repopilot review --head` requires --base. Add `--base origin/main` (replace `origin/main` with your target branch); for a branch range, use `--base origin/main --head HEAD`. Omit `--head` to compare the working tree against `HEAD`.",
        ));
    }
    if options.since_snapshot && (options.base.is_some() || options.head.is_some()) {
        return Err(usage_error(
            "`--since-snapshot` cannot be used with --base or --head",
        ));
    }
    Ok(())
}

fn usage_error(message: &str) -> CliExit {
    CliExit {
        code: EXIT_USAGE,
        message: message.to_string(),
    }
}

fn load_config(options: &ReviewOptions) -> Result<RepoPilotConfig, ReviewConfigError> {
    match &options.config {
        Some(path) => load_optional_config(path),
        None => load_default_config(),
    }
    .map_err(ReviewConfigError)
}

fn review_scope(options: &ReviewOptions, config: &RepoPilotConfig) -> ReviewScope {
    match options.scope {
        Some(ReviewScopeArg::Changed) => ReviewScope::Changed,
        Some(ReviewScopeArg::Full) => ReviewScope::Full,
        None => config.review.scope,
    }
}

fn visibility_profile(options: &ReviewOptions, scope: ReviewScope) -> FindingVisibilityProfile {
    match options.profile {
        Some(ScanProfileArg::Default) => FindingVisibilityProfile::Default,
        Some(ScanProfileArg::Strict) => FindingVisibilityProfile::Strict,
        None if scope == ReviewScope::Full => FindingVisibilityProfile::Strict,
        None => FindingVisibilityProfile::Default,
    }
}

fn review_gate_policy(options: &ReviewOptions, config: &RepoPilotConfig) -> ReviewSignalGatePolicy {
    match options.fail_on_review {
        Some(ReviewFailOnArg::None) => ReviewSignalGatePolicy::None,
        Some(ReviewFailOnArg::Definitely) => ReviewSignalGatePolicy::Definitely,
        None => match config.review.fail_on {
            ReviewFailOn::None => ReviewSignalGatePolicy::None,
            ReviewFailOn::Definitely => ReviewSignalGatePolicy::Definitely,
        },
    }
}

fn load_input(
    options: &ReviewOptions,
) -> Result<(repopilot::review::ReviewInput, u64), Box<dyn Error>> {
    let started = Instant::now();
    let input = if options.since_snapshot {
        let snapshot = crate::commands::snapshot::read_snapshot(&options.path)?;
        load_review_input_since(&options.path, snapshot.review_base())?
    } else {
        load_review_input(
            &options.path,
            options.base.as_deref(),
            options.head.as_deref(),
        )?
    };
    Ok((input, duration_us(started.elapsed())))
}

fn load_intent(
    options: &ReviewOptions,
    input: &repopilot::review::ReviewInput,
    config: &RepoPilotConfig,
) -> Result<IntentContext, Box<dyn Error>> {
    let contract = options
        .intent
        .as_deref()
        .map(|path| load_intent_file(&input.repo_root, path, config))
        .transpose()?;
    Ok(IntentContext {
        contract,
        critical_paths: config.review.critical_paths.clone(),
    })
}

pub(super) fn scan(
    options: &ReviewOptions,
    plan: &ReviewPlan,
) -> Result<crate::commands::product_scan::ProductScanResult, Box<dyn Error>> {
    let mode = match plan.scope {
        ReviewScope::Changed => ProductScanMode::ResolvedChanged {
            changed_files: plan.review_input.changed_files.clone(),
            base_ref: plan.review_input.target.base_ref().map(str::to_string),
        },
        ReviewScope::Full => ProductScanMode::Full,
    };
    run_product_scan(ProductScanRequest {
        path: options.path.clone(),
        config_path: options.config.clone(),
        overrides: ScanConfigOverrides {
            max_file_loc: options.max_file_loc,
            max_directory_modules: options.max_directory_modules,
            max_directory_depth: options.max_directory_depth,
            ..ScanConfigOverrides::default()
        },
        preset: None,
        mode,
        no_progress: options.no_progress,
        ignore_feedback: options.ignore_feedback,
        visibility_profile: plan.visibility_profile,
        pre_visibility_filter: review_pre_diff_filter(options),
    })
}

pub(super) fn build_report(
    options: &ReviewOptions,
    plan: &ReviewPlan,
    summary: ScanSummary,
    session: &AnalysisSession,
) -> Result<ReviewReport, Box<dyn Error>> {
    let baseline_file = match options.baseline.as_ref() {
        Some(path) => Some((
            repopilot::baseline::reader::read_baseline(path)?,
            path.clone(),
        )),
        None => None,
    };
    let baseline_ref = baseline_file
        .as_ref()
        .map(|(baseline, path)| (baseline, path.clone()));
    let started = Instant::now();
    let mut report = build_review_report_from_session(
        summary,
        plan.review_input.clone(),
        baseline_ref,
        session,
    )?;
    report.intent = plan.intent.clone();
    report.timings.diff_loading_us = plan.diff_loading_us;
    report.timings.review_signals_us = duration_us(started.elapsed());
    if plan.scope == ReviewScope::Changed {
        report.retain_in_diff_findings();
    }
    if !plan.priority_filter.is_empty() {
        report.apply_filter(&plan.priority_filter);
    }
    Ok(report)
}

pub(super) fn explain_signal(
    options: &ReviewOptions,
    report: &ReviewReport,
    policy: ReviewSignalGatePolicy,
) -> Result<bool, Box<dyn Error>> {
    let Some(signal_id) = options.explain_signal.as_deref() else {
        return Ok(false);
    };
    let gate = ReviewSignalGateResult::evaluate(report, policy);
    let report_json = render_with_options(
        report,
        OutputFormat::Json,
        None,
        Some(&gate),
        ReviewRenderOptions::full(),
    )?;
    let explanation = crate::commands::mcp::explain_signal_from_review(
        &report_json,
        signal_id,
        options.explain_signal_path.as_deref(),
        options.explain_signal_line,
    )
    .map_err(|message| CliExit {
        code: EXIT_USAGE,
        message,
    })?;
    write_report(&explanation, options.output.as_deref())?;
    Ok(true)
}

#[derive(Debug)]
struct ReviewConfigError(ConfigError);

impl fmt::Display for ReviewConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}. Next step: fix this config or rerun with --config PATH to select another valid file.",
            self.0
        )
    }
}

impl Error for ReviewConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}
