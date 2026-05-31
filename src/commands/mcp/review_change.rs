//! The `repopilot_review_change` MCP tool: the local "is this change risky?"
//! audit, wrapping the same scan + review pipeline the `review` command uses.

use crate::commands::product_scan::{ProductScanMode, ProductScanRequest, run_product_scan};
use crate::commands::scan_config::ScanConfigOverrides;
use repopilot::findings::filter::FindingFilter;
use repopilot::findings::visibility::FindingVisibilityProfile;
use repopilot::output::OutputFormat;
use repopilot::review::build_review_report;
use repopilot::review::render::render;
use serde_json::{Value, json};
use std::path::PathBuf;

pub const TOOL_NAME: &str = "repopilot_review_change";

/// The `tools/list` descriptor for this tool.
pub fn definition() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": "Audit the current Git changes locally. Scans the repository, splits findings into those touching changed diff lines vs the rest, and reports blast radius (files that import the changed files). Runs entirely on disk — nothing is uploaded. Returns a JSON review report.",
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
                }
            },
            "additionalProperties": false
        }
    })
}

/// Runs the review for a `tools/call`, returning the JSON report on success or a
/// human-readable message on failure (surfaced to the agent as an error result).
pub fn call(arguments: &Value) -> Result<String, String> {
    let path = PathBuf::from(arguments.get("path").and_then(Value::as_str).unwrap_or("."));
    let base = arguments.get("base").and_then(Value::as_str);
    let head = arguments.get("head").and_then(Value::as_str);

    if base.is_none() && head.is_some() {
        return Err("`head` requires `base`".to_string());
    }

    let scan_result = run_product_scan(ProductScanRequest {
        path: path.clone(),
        config_path: None,
        overrides: ScanConfigOverrides::default(),
        preset: None,
        mode: ProductScanMode::Full,
        // The stdio transport owns stdout; progress would corrupt the JSON-RPC
        // stream, so it is always disabled here.
        no_progress: true,
        ignore_feedback: false,
        visibility_profile: FindingVisibilityProfile::Strict,
        pre_visibility_filter: FindingFilter::default(),
    })
    .map_err(|error| format!("scan failed: {error}"))?;

    let review_report = build_review_report(scan_result.summary, &path, base, head, None)
        .map_err(|error| format!("review failed: {error}"))?;

    render(&review_report, OutputFormat::Json, None)
        .map_err(|error| format!("render failed: {error}"))
}
