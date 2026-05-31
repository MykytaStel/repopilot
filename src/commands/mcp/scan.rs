//! The `repopilot_scan` MCP tool: a full repository audit, wrapping the same
//! scan pipeline the `scan` command uses and returning the JSON report.

use crate::commands::product_scan::{ProductScanMode, ProductScanRequest, run_product_scan};
use crate::commands::scan_config::ScanConfigOverrides;
use repopilot::findings::filter::FindingFilter;
use repopilot::findings::visibility::FindingVisibilityProfile;
use repopilot::output::{OutputFormat, render_scan_summary};
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
                }
            },
            "additionalProperties": false
        }
    })
}

pub fn call(arguments: &Value) -> Result<String, String> {
    let path = PathBuf::from(arguments.get("path").and_then(Value::as_str).unwrap_or("."));

    let scan_result = run_product_scan(ProductScanRequest {
        path,
        config_path: None,
        overrides: ScanConfigOverrides::default(),
        preset: None,
        mode: ProductScanMode::Full,
        // The stdio transport owns stdout; progress would corrupt the JSON-RPC
        // stream, so it is always disabled here.
        no_progress: true,
        ignore_feedback: false,
        visibility_profile: FindingVisibilityProfile::Default,
        pre_visibility_filter: FindingFilter::default(),
    })
    .map_err(|error| format!("scan failed: {error}"))?;

    render_scan_summary(&scan_result.summary, OutputFormat::Json)
        .map_err(|error| format!("render failed: {error}"))
}
