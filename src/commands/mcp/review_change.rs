//! The `repopilot_review_change` MCP tool: the local "is this change risky?"
//! audit, wrapping the same scan + review pipeline the `review` command uses.

use super::verification_consent::{self, EffectiveConfigSource};
use crate::commands::mcp::review_projection::compact_review_json;
use crate::commands::product_scan::{ProductScanMode, ProductScanRequest, run_product_scan};
use crate::commands::review_verification::{
    ReviewVerificationEvent, VerificationApproval, run_selected_with_approval,
};
use crate::commands::scan_config::ScanConfigOverrides;
use repopilot::baseline::reader::read_baseline;
use repopilot::findings::filter::FindingFilter;
use repopilot::findings::visibility::FindingVisibilityProfile;
use repopilot::output::OutputFormat;
use repopilot::review::intent::{
    IntentContext, load_intent_file, parse_intent_json, validate_critical_paths, validate_intent,
};
use repopilot::review::render::render;
use repopilot::review::{
    ReviewSignalGatePolicy, ReviewSignalGateResult, build_review_report_from_session,
    load_review_input,
};
use repopilot::verification::{CancellationToken, ValidatedCheck};
use serde_json::Value;
use std::fmt;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub const TOOL_NAME: &str = "repopilot_review_change";

pub(super) struct ReviewCallContext<'a> {
    pub cancellation: &'a CancellationToken,
    pub observer: &'a mut dyn FnMut(ReviewVerificationEvent),
    pub approve: &'a mut dyn FnMut(&ValidatedCheck) -> VerificationApproval,
}

pub(super) struct ReviewCallResult {
    pub rendered: String,
    pub analysis_revision: String,
    pub revision_compatible: bool,
}

#[derive(Debug)]
pub(super) enum ReviewCallError {
    Cancelled,
    Message(String),
}

impl fmt::Display for ReviewCallError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => formatter.write_str("request cancelled"),
            Self::Message(message) => formatter.write_str(message),
        }
    }
}

impl From<String> for ReviewCallError {
    fn from(message: String) -> Self {
        Self::Message(message)
    }
}

mod definition;
pub use definition::definition;

/// Runs the review for a `tools/call`, returning the JSON report on success or a
/// human-readable message on failure (surfaced to the agent as an error result).
pub(super) fn call_with_context(
    arguments: &Value,
    context: ReviewCallContext<'_>,
) -> Result<ReviewCallResult, ReviewCallError> {
    let selected_checks = verification_ids(arguments)?;
    if context.cancellation.is_cancelled() {
        return Err(ReviewCallError::Cancelled);
    }
    let path = PathBuf::from(arguments.get("path").and_then(Value::as_str).unwrap_or("."));
    let base = arguments.get("base").and_then(Value::as_str);
    let head = arguments.get("head").and_then(Value::as_str);
    let config_path = arguments
        .get("config")
        .and_then(Value::as_str)
        .map(PathBuf::from);
    let baseline_path = arguments
        .get("baseline")
        .and_then(Value::as_str)
        .map(PathBuf::from);
    let intent_path = arguments
        .get("intent_path")
        .and_then(Value::as_str)
        .map(PathBuf::from);
    if intent_path.is_some() && arguments.get("intent").is_some() {
        return Err(ReviewCallError::Message(
            "`intent` and `intent_path` cannot be used together".to_string(),
        ));
    }

    if base.is_none() && head.is_some() {
        return Err(ReviewCallError::Message(
            "`head` requires `base`".to_string(),
        ));
    }

    let diff_started = Instant::now();
    let input =
        load_review_input(&path, base, head).map_err(|error| format!("review failed: {error}"))?;
    let review_target = input.target.clone();
    let config_source = EffectiveConfigSource::resolve(config_path.as_deref(), &input.repo_root);
    let configured = verification_consent::load_initial_config(&config_source)?;
    validate_critical_paths(&configured).map_err(|error| error.to_string())?;
    let intent = match (arguments.get("intent"), intent_path.as_deref()) {
        (Some(value), None) => {
            let intent = parse_intent_json(value).map_err(|error| error.to_string())?;
            validate_intent(&intent, &configured).map_err(|error| error.to_string())?;
            Some(intent)
        }
        (None, Some(path)) => Some(
            load_intent_file(&input.repo_root, path, &configured)
                .map_err(|error| error.to_string())?,
        ),
        (None, None) => None,
        (Some(_), Some(_)) => unreachable!("intent paths were rejected above"),
    };
    let diff_loading_us = duration_us(diff_started.elapsed());
    let scope = arguments
        .get("scope")
        .and_then(Value::as_str)
        .unwrap_or("changed");
    let visibility_profile = match arguments.get("profile").and_then(Value::as_str) {
        Some("strict") => FindingVisibilityProfile::Strict,
        Some("default") | None => FindingVisibilityProfile::Default,
        Some(other) => return Err(format!("invalid profile: {other}").into()),
    };
    let mode = match scope {
        "changed" => ProductScanMode::ResolvedChanged {
            changed_files: input.changed_files.clone(),
            base_ref: input.target.base_ref().map(str::to_string),
        },
        "full" => ProductScanMode::Full,
        other => return Err(format!("invalid scope: {other}").into()),
    };
    let filter = super::scan::parse_filters(arguments)?;

    let scan_result = run_product_scan(ProductScanRequest {
        path: path.clone(),
        config_path,
        overrides: ScanConfigOverrides::default(),
        preset: None,
        mode,
        // The stdio transport owns stdout; progress would corrupt the JSON-RPC
        // stream, so it is always disabled here.
        no_progress: true,
        ignore_feedback: false,
        visibility_profile,
        pre_visibility_filter: FindingFilter {
            min_priority: None,
            ..filter.clone()
        },
    })
    .map_err(|error| format!("scan failed: {error}"))?;

    let baseline_file = baseline_path
        .map(|baseline_path| {
            read_baseline(&baseline_path)
                .map(|baseline| (baseline, baseline_path))
                .map_err(|error| format!("baseline failed: {error}"))
        })
        .transpose()?;
    let baseline_ref = baseline_file
        .as_ref()
        .map(|(baseline, path)| (baseline, path.clone()));
    let review_started = Instant::now();
    let mut review_report = build_review_report_from_session(
        scan_result.summary,
        input,
        baseline_ref,
        &scan_result.session,
    )
    .map_err(|error| format!("review failed: {error}"))?;
    review_report.intent = IntentContext {
        contract: intent,
        critical_paths: configured.review.critical_paths.clone(),
    };
    let analysis_revision = scan_result.session.revision().id().to_string();
    review_report.timings.diff_loading_us = diff_loading_us;
    review_report.timings.review_signals_us = duration_us(review_started.elapsed());
    if scope == "changed" {
        review_report.retain_in_diff_findings();
    }
    if filter.min_priority.is_some() {
        review_report.apply_filter(&FindingFilter {
            min_priority: filter.min_priority,
            ..FindingFilter::default()
        });
    }
    if context.cancellation.is_cancelled() {
        return Err(ReviewCallError::Cancelled);
    }
    let mut reload_check = |check_id: &str| {
        verification_consent::reload_check(
            scan_result.session.workspace_root(),
            &config_source,
            check_id,
        )
    };
    run_selected_with_approval(
        &selected_checks,
        &scan_result.session,
        &review_target,
        &mut review_report,
        context.cancellation,
        context.observer,
        &mut reload_check,
        context.approve,
    )
    .map_err(|error| ReviewCallError::Message(error.to_string()))?;
    if context.cancellation.is_cancelled() {
        return Err(ReviewCallError::Cancelled);
    }
    let gate_policy = match arguments
        .get("fail_on_review")
        .and_then(Value::as_str)
        .unwrap_or("none")
    {
        "none" => ReviewSignalGatePolicy::None,
        "definitely" => ReviewSignalGatePolicy::Definitely,
        other => return Err(format!("invalid fail_on_review: {other}").into()),
    };
    let gating_started = Instant::now();
    let review_gate = ReviewSignalGateResult::evaluate(&review_report, gate_policy);
    review_report.timings.gating_us = duration_us(gating_started.elapsed());
    let rendering_started = Instant::now();
    let _ = render(&review_report, OutputFormat::Json, None, Some(&review_gate))
        .map_err(|error| format!("render failed: {error}"))?;
    review_report.timings.rendering_us = duration_us(rendering_started.elapsed());
    let rendered = render(&review_report, OutputFormat::Json, None, Some(&review_gate))
        .map_err(|error| format!("render failed: {error}"))?;

    let rendered = if arguments
        .get("detail")
        .and_then(Value::as_str)
        .is_some_and(|detail| detail == "full")
    {
        rendered
    } else {
        compact_review_json(&rendered)?
    };
    Ok(ReviewCallResult {
        rendered,
        analysis_revision,
        revision_compatible: review_report
            .verification
            .iter()
            .all(|outcome| outcome.revision_compatible),
    })
}

fn verification_ids(arguments: &Value) -> Result<Vec<String>, ReviewCallError> {
    let Some(value) = arguments.get("verify") else {
        return Ok(Vec::new());
    };
    let values = value.as_array().ok_or_else(|| {
        ReviewCallError::Message("`verify` must be an array of strings".to_string())
    })?;
    values
        .iter()
        .map(|value| {
            value.as_str().map(str::to_string).ok_or_else(|| {
                ReviewCallError::Message("`verify` must be an array of strings".to_string())
            })
        })
        .collect()
}

fn duration_us(duration: Duration) -> u64 {
    duration.as_micros().min(u128::from(u64::MAX)) as u64
}

#[cfg(test)]
mod tests;
