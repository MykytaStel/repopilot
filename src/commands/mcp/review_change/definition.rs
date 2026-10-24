//! The `tools/list` descriptor for `repopilot_review_change`.

use super::TOOL_NAME;
use serde_json::{Map, Value, json};

/// The `tools/list` descriptor for this tool.
pub fn definition() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": concat!(
            "Review a Git change locally: what it touched and which checks it weakened. Use it before finishing work or before a merge. By default it reviews uncommitted work (working tree vs HEAD); pass `base` (e.g. \"origin/main\") to review a branch. For a repository health check that is not about one change, use repopilot_scan instead. ",
            "The text block is concise Markdown; structuredContent contains the machine-readable JSON report with the decision, changed and existing findings, blast radius, verification, and confidence-tiered signals. It covers weakened tests and gates; security-boundary changes; behavioral changes, including a changed quiet-fallback candidate in parsed JS/TS or Python; a removed named TypeScript/JavaScript export that a resolved local caller still imports; algorithmic shifts; and taint-lite paths from HTTP or process input to SQL, exec, file-write, or outbound-network sinks. ",
            "Treat quiet-fallback signals as candidates to verify, not proof of harmful behavior, and preserve intentional recovery paths. Explain one with repopilot_explain_review_signal. RepoPilot uploads nothing. Only a non-empty `verify` array selects configured local checks. Each selected check runs only after one-time form-mode Elicitation consent from a client that negotiated protocol `2025-11-25` and `elicitation.form`; otherwise it is skipped. The prompt shows the command, exact arguments, and working directory. The process has the host user’s permissions and is not sandboxed; its captured output is bounded and redacted."
        ),
        "inputSchema": input_schema(),
        "outputSchema": { "type": "object", "additionalProperties": true },
        "annotations": {
            "readOnlyHint": false,
            "destructiveHint": true,
            "idempotentHint": false,
            "openWorldHint": true
        }
    })
}

fn input_schema() -> Value {
    let mut properties = repository_properties();
    properties.extend(review_properties());
    properties.insert("intent".to_string(), intent_property());
    properties.insert("filters".to_string(), filters_property());
    json!({
        "type": "object",
        "properties": properties,
        "additionalProperties": false
    })
}

fn repository_properties() -> Map<String, Value> {
    properties_from(json!({
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
        "config": {
            "type": "string",
            "description": "Optional repopilot.toml path. Defaults to the one discovered in the repository."
        },
        "intent_path": {
            "type": "string",
            "description": "Optional repository-rooted TOML file that states what the change is meant to touch. The report marks drift outside it. Use either this or `intent`, not both."
        },
        "baseline": {
            "type": "string",
            "description": "Optional baseline file (from `repopilot baseline create`). Findings recorded in it count as accepted debt, separate from new findings."
        }
    }))
}

fn review_properties() -> Map<String, Value> {
    properties_from(json!({
        "scope": {
            "type": "string",
            "enum": ["changed", "full"],
            "default": "changed",
            "description": "\"changed\" reports findings in the changed files only; \"full\" also returns findings from the rest of the repository."
        },
        "profile": {
            "type": "string",
            "enum": ["default", "strict"],
            "default": "default",
            "description": "\"default\" hides low-signal suggestions; \"strict\" shows all findings."
        },
        "fail_on_review": {
            "type": "string",
            "enum": ["none", "definitely"],
            "default": "none",
            "description": "\"definitely\" fails the report's gate when a gate-eligible definitely-tier signal is present; \"none\" reports signals without gating."
        },
        "detail": {
            "type": "string",
            "enum": ["compact", "full"],
            "default": "compact",
            "description": "\"compact\" returns at most 20 findings and 20 signals; \"full\" returns all of them."
        },
        "offset": { "type": "integer", "minimum": 0, "description": "Zero-based finding offset." },
        "limit": { "type": "integer", "minimum": 1, "maximum": 1000, "description": "Maximum findings to return." },
        "verify": {
            "type": "array",
            "items": { "type": "string" },
            "uniqueItems": true,
            "description": "IDs of checks configured in repopilot.toml to request for the reviewed revision. Each needs a separate one-time user approval through form-mode Elicitation; unsupported clients skip checks. Omit to run nothing."
        }
    }))
}

fn intent_property() -> Value {
    json!({
        "type": "object",
        "description": "Optional inline statement of what the change is meant to touch (paths, contract families, critical paths, verification IDs). The report marks drift outside it. It is metadata only and cannot execute commands.",
        "properties": {
            "version": { "type": "integer", "const": 1 },
            "summary": { "type": "string", "maxLength": 256 },
            "paths": { "type": "array", "items": { "type": "string", "maxLength": 256 }, "maxItems": 32 },
            "contract_families": { "type": "array", "items": { "type": "string" }, "maxItems": 32 },
            "critical_paths": { "type": "array", "items": { "type": "string", "maxLength": 256 }, "maxItems": 32 },
            "verification": { "type": "array", "items": { "type": "string", "maxLength": 256 }, "maxItems": 32 }
        },
        "additionalProperties": false
    })
}

fn filters_property() -> Value {
    json!({
        "type": "object",
        "description": "Optional thresholds and rule IDs that narrow the returned findings.",
        "properties": {
            "min_severity": { "type": "string", "enum": ["info", "low", "medium", "high", "critical"] },
            "min_confidence": { "type": "string", "enum": ["low", "medium", "high"] },
            "min_priority": { "type": "string", "enum": ["p0", "p1", "p2", "p3"] },
            "rules": { "type": "array", "items": { "type": "string" } }
        },
        "additionalProperties": false
    })
}

fn properties_from(value: Value) -> Map<String, Value> {
    value
        .as_object()
        .expect("input schema property groups are objects")
        .clone()
}
