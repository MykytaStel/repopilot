//! The `repopilot_context` MCP tool: a budgeted, AI-ready Markdown brief of the
//! repository, wrapping the same builder the `ai context` command uses.

use crate::commands::focus::parse_focus_category;
use crate::commands::product_scan::{ProductScanMode, ProductScanRequest, run_product_scan};
use crate::commands::scan_config::ScanConfigOverrides;
use repopilot::findings::filter::FindingFilter;
use repopilot::findings::visibility::FindingVisibilityProfile;
use repopilot::output::ai_context::{AiContextRenderOptions, DEFAULT_TOKEN_BUDGET, render};
use serde_json::{Value, json};
use std::path::PathBuf;

pub const TOOL_NAME: &str = "repopilot_context";

pub fn definition() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": "Generate a budgeted, AI-ready Markdown brief of the repository (risks, hotspots, structure) for an agent to reason over before editing. Built locally from a scan — no AI service is called and nothing is uploaded.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Repository path. Defaults to the current working directory."
                },
                "focus": {
                    "type": "string",
                    "description": "Optional focus: security, architecture (or arch), quality, framework, or all."
                },
                "budget": {
                    "type": "integer",
                    "description": "Optional approximate token budget for the brief. Defaults to the standard budget."
                }
            },
            "additionalProperties": false
        }
    })
}

pub fn call(arguments: &Value) -> Result<String, String> {
    let path = PathBuf::from(arguments.get("path").and_then(Value::as_str).unwrap_or("."));
    let focus = parse_focus_category(arguments.get("focus").and_then(Value::as_str))
        .map_err(|error| error.to_string())?;
    let budget_tokens = arguments
        .get("budget")
        .and_then(Value::as_u64)
        .map_or(DEFAULT_TOKEN_BUDGET, |budget| budget as usize);

    let scan_result = run_product_scan(ProductScanRequest {
        path,
        config_path: None,
        overrides: ScanConfigOverrides::default(),
        preset: None,
        mode: ProductScanMode::Full,
        no_progress: true,
        ignore_feedback: false,
        visibility_profile: FindingVisibilityProfile::Default,
        pre_visibility_filter: FindingFilter::default(),
    })
    .map_err(|error| format!("scan failed: {error}"))?;

    let options = AiContextRenderOptions {
        focus,
        budget_tokens,
        no_header: false,
        no_task: false,
    };

    Ok(render(&scan_result.summary, &options))
}
