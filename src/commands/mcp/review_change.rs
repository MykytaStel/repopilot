//! The `repopilot_review_change` MCP tool: the local "is this change risky?"
//! audit, wrapping the same scan + review pipeline the `review` command uses.

use crate::commands::product_scan::{ProductScanMode, ProductScanRequest, run_product_scan};
use crate::commands::scan_config::ScanConfigOverrides;
use repopilot::baseline::reader::read_baseline;
use repopilot::findings::filter::FindingFilter;
use repopilot::findings::visibility::FindingVisibilityProfile;
use repopilot::output::OutputFormat;
use repopilot::review::render::render;
use repopilot::review::{
    ReviewSignalGatePolicy, ReviewSignalGateResult, build_review_report_from_session,
    load_review_input,
};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub const TOOL_NAME: &str = "repopilot_review_change";

/// The `tools/list` descriptor for this tool.
pub fn definition() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": "Audit the current Git changes locally. Scans the repository, splits findings into those touching changed diff lines vs the rest, and reports blast radius (files that import the changed files). Also surfaces deterministic change signals grouped by confidence tier (definitely-sensitive / maybe-sensitive / large-diff-or-noise) on `tiered_signals`: security-boundary changes (auth, CORS, CI, dependency manifests, committed .env), behavioral changes (network/subprocess/filesystem/env/dependency/migration/raw-SQL added; error-handling, auth-check, or test removed), algorithmic changes (control-flow nesting deeper, nested loop introduced, function grew, recursion introduced), and taint-lite reachability (HTTP request or process arguments reaching SQL, exec, filesystem-write, or outbound-network sinks within a changed function). These flag, they do not judge. Runs entirely on disk — nothing is uploaded. Returns a JSON review report.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Repository path to review. Defaults to the current working directory."
                },
                "base": {
                    "type": "string",
                    "description": "Base Git ref to diff against, e.g. \"origin/main\". Optional; defaults to the working tree vs HEAD."
                },
                "head": {
                    "type": "string",
                    "description": "Head Git ref. Optional and only valid together with \"base\"."
                },
                "config": { "type": "string", "description": "Optional repopilot.toml path." },
                "baseline": { "type": "string", "description": "Optional baseline path." },
                "scope": { "type": "string", "enum": ["changed", "full"], "default": "changed" },
                "profile": { "type": "string", "enum": ["default", "strict"], "default": "default" },
                "fail_on_review": { "type": "string", "enum": ["none", "definitely"], "default": "none" },
                "detail": { "type": "string", "enum": ["compact", "full"], "default": "compact" },
                "filters": {
                    "type": "object",
                    "properties": {
                        "min_severity": { "type": "string", "enum": ["info", "low", "medium", "high", "critical"] },
                        "min_confidence": { "type": "string", "enum": ["low", "medium", "high"] },
                        "min_priority": { "type": "string", "enum": ["p0", "p1", "p2", "p3"] },
                        "rules": { "type": "array", "items": { "type": "string" } }
                    },
                    "additionalProperties": false
                }
            },
            "additionalProperties": false
        },
        "outputSchema": { "type": "object", "additionalProperties": true },
        "annotations": {
            "readOnlyHint": true,
            "destructiveHint": false,
            "idempotentHint": true,
            "openWorldHint": false
        }
    })
}

/// Runs the review for a `tools/call`, returning the JSON report on success or a
/// human-readable message on failure (surfaced to the agent as an error result).
pub fn call(arguments: &Value) -> Result<String, String> {
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

    if base.is_none() && head.is_some() {
        return Err("`head` requires `base`".to_string());
    }

    let diff_started = Instant::now();
    let input =
        load_review_input(&path, base, head).map_err(|error| format!("review failed: {error}"))?;
    let diff_loading_us = duration_us(diff_started.elapsed());
    let scope = arguments
        .get("scope")
        .and_then(Value::as_str)
        .unwrap_or("changed");
    let visibility_profile = match arguments.get("profile").and_then(Value::as_str) {
        Some("strict") => FindingVisibilityProfile::Strict,
        Some("default") | None => FindingVisibilityProfile::Default,
        Some(other) => return Err(format!("invalid profile: {other}")),
    };
    let mode = match scope {
        "changed" => ProductScanMode::ResolvedChanged {
            changed_files: input.changed_files.clone(),
            base_ref: input.target.base_ref().map(str::to_string),
        },
        "full" => ProductScanMode::Full,
        other => return Err(format!("invalid scope: {other}")),
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
    let gate_policy = match arguments
        .get("fail_on_review")
        .and_then(Value::as_str)
        .unwrap_or("none")
    {
        "none" => ReviewSignalGatePolicy::None,
        "definitely" => ReviewSignalGatePolicy::Definitely,
        other => return Err(format!("invalid fail_on_review: {other}")),
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

    if arguments
        .get("detail")
        .and_then(Value::as_str)
        .is_some_and(|detail| detail == "full")
    {
        return Ok(rendered);
    }
    compact_review_json(&rendered)
}

fn duration_us(duration: Duration) -> u64 {
    duration.as_micros().min(u128::from(u64::MAX)) as u64
}

fn compact_review_json(rendered: &str) -> Result<String, String> {
    const LIMIT: usize = 20;
    let mut value: Value =
        serde_json::from_str(rendered).map_err(|error| format!("compact failed: {error}"))?;
    if let Some(findings) = value.get_mut("findings").and_then(Value::as_array_mut) {
        findings.truncate(LIMIT);
    }
    let mut remaining = LIMIT;
    for tier in ["definitely", "maybe", "noise"] {
        if let Some(signals) = value
            .get_mut("tiered_signals")
            .and_then(|tiered| tiered.get_mut(tier))
            .and_then(Value::as_array_mut)
        {
            signals.truncate(remaining);
            remaining = remaining.saturating_sub(signals.len());
        }
    }
    serde_json::to_string_pretty(&value).map_err(|error| format!("compact failed: {error}"))
}
