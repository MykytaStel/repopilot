//! The `repopilot_scan` MCP tool: a full repository audit, wrapping the same
//! scan pipeline the `scan` command uses and returning the JSON report.

use crate::commands::product_scan::{ProductScanMode, ProductScanRequest, run_product_scan};
use crate::commands::scan_config::ScanConfigOverrides;
use repopilot::findings::filter::FindingFilter;
use repopilot::findings::types::{Confidence, Severity};
use repopilot::findings::visibility::FindingVisibilityProfile;
use repopilot::output::{OutputFormat, render_scan_summary};
use repopilot::risk::RiskPriority;
use serde_json::{Value, json};
use std::path::PathBuf;

pub const TOOL_NAME: &str = "repopilot_scan";

pub fn definition() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": "Audit a repository, folder, or file for findings across architecture, coupling, code quality, security, and testing, and return the full JSON scan report (findings, metrics, risk summary). Runs entirely on disk — nothing is uploaded.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to scan. Defaults to the current working directory."
                },
                "config": { "type": "string", "description": "Optional repopilot.toml path." },
                "profile": { "type": "string", "enum": ["default", "strict"], "default": "default" },
                "scope": { "type": "string", "enum": ["full", "changed"], "default": "full" },
                "base": { "type": "string", "description": "Optional base ref for changed scope." },
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

pub fn call(arguments: &Value) -> Result<String, String> {
    let path = PathBuf::from(arguments.get("path").and_then(Value::as_str).unwrap_or("."));
    let config_path = arguments
        .get("config")
        .and_then(Value::as_str)
        .map(PathBuf::from);
    let visibility_profile = match arguments.get("profile").and_then(Value::as_str) {
        Some("strict") => FindingVisibilityProfile::Strict,
        Some("default") | None => FindingVisibilityProfile::Default,
        Some(other) => return Err(format!("invalid profile: {other}")),
    };
    let mode = match arguments.get("scope").and_then(Value::as_str) {
        Some("changed") => ProductScanMode::Changed {
            since: arguments
                .get("base")
                .and_then(Value::as_str)
                .map(str::to_string),
        },
        Some("full") | None => ProductScanMode::Full,
        Some(other) => return Err(format!("invalid scope: {other}")),
    };

    let filter = parse_filters(arguments)?;
    let mut scan_result = run_product_scan(ProductScanRequest {
        path,
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
    if filter.min_priority.is_some() {
        filter.apply_to_summary(&mut scan_result.summary);
    }

    render_scan_summary(&scan_result.summary, OutputFormat::Json)
        .map_err(|error| format!("render failed: {error}"))
}

pub(super) fn parse_filters(arguments: &Value) -> Result<FindingFilter, String> {
    let filters = arguments.get("filters").unwrap_or(&Value::Null);
    let min_severity = match filters.get("min_severity").and_then(Value::as_str) {
        None => None,
        Some("info") => Some(Severity::Info),
        Some("low") => Some(Severity::Low),
        Some("medium") => Some(Severity::Medium),
        Some("high") => Some(Severity::High),
        Some("critical") => Some(Severity::Critical),
        Some(value) => return Err(format!("invalid min_severity: {value}")),
    };
    let min_confidence = match filters.get("min_confidence").and_then(Value::as_str) {
        None => None,
        Some("low") => Some(Confidence::Low),
        Some("medium") => Some(Confidence::Medium),
        Some("high") => Some(Confidence::High),
        Some(value) => return Err(format!("invalid min_confidence: {value}")),
    };
    let min_priority = match filters.get("min_priority").and_then(Value::as_str) {
        None => None,
        Some("p0") => Some(RiskPriority::P0),
        Some("p1") => Some(RiskPriority::P1),
        Some("p2") => Some(RiskPriority::P2),
        Some("p3") => Some(RiskPriority::P3),
        Some(value) => return Err(format!("invalid min_priority: {value}")),
    };
    let rule_ids = filters
        .get("rules")
        .and_then(Value::as_array)
        .map(|rules| {
            rules
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    Ok(FindingFilter {
        min_severity,
        min_confidence,
        min_priority,
        rule_ids,
    })
}
