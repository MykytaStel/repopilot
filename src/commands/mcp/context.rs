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
                },
                "config": { "type": "string", "description": "Optional repopilot.toml path." },
                "profile": { "type": "string", "enum": ["default", "strict"], "default": "default" }
            },
            "additionalProperties": false
        },
        "outputSchema": {
            "type": "object",
            "properties": { "markdown": { "type": "string" } },
            "required": ["markdown"],
            "additionalProperties": false
        },
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
    let focus = parse_focus_category(arguments.get("focus").and_then(Value::as_str))
        .map_err(|error| error.to_string())?;
    let budget_tokens = arguments
        .get("budget")
        .and_then(Value::as_u64)
        .map_or(DEFAULT_TOKEN_BUDGET, |budget| budget as usize);
    let config_path = arguments
        .get("config")
        .and_then(Value::as_str)
        .map(PathBuf::from);
    let visibility_profile = match arguments.get("profile").and_then(Value::as_str) {
        Some("strict") => FindingVisibilityProfile::Strict,
        Some("default") | None => FindingVisibilityProfile::Default,
        Some(other) => return Err(format!("invalid profile: {other}")),
    };

    let scan_result = run_product_scan(ProductScanRequest {
        path,
        config_path,
        overrides: ScanConfigOverrides::default(),
        preset: None,
        mode: ProductScanMode::Full,
        no_progress: true,
        ignore_feedback: false,
        visibility_profile,
        pre_visibility_filter: FindingFilter::default(),
    })
    .map_err(|error| format!("scan failed: {error}"))?;

    // Agents that call this tool bring their own instructions, so return the
    // fact-only context (facts, evidence, prioritized plan, edit order) — the
    // same form `repopilot ai context --no-task` emits — and omit the human task
    // preamble, working rules, and verification checklist.
    let options = AiContextRenderOptions {
        focus,
        budget_tokens,
        no_header: false,
        no_task: true,
    };

    Ok(render(&scan_result.summary, &options))
}
