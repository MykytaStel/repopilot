//! The `tools/list` descriptor for `repopilot_review_change`.

use super::TOOL_NAME;
use serde_json::{Value, json};

/// The `tools/list` descriptor for this tool.
pub fn definition() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": "Review a Git change locally: what it touched and which checks it weakened. Use it before finishing work or before a merge. By default it reviews uncommitted work (working tree vs HEAD); pass `base` (e.g. \"origin/main\") to review a branch. For a repository health check that is not about one change, use repopilot_scan instead. Returns a JSON report with a decision, findings on changed lines vs the rest, blast radius (files that import the changed files), and deterministic signals grouped by confidence tier (definitely / maybe / noise) in `tiered_signals`: checks the change weakened (focused or skipped tests, removed tests, tests that lost assertions, new lint/type/coverage suppressions, relaxed CI or tool gates); security-boundary changes (auth, CORS, CI, dependency manifests, committed .env); behavioral changes (network, subprocess, filesystem, env, dependency, migration, or raw SQL added; error handling, an auth check, or a test removed; a removed named TypeScript/JavaScript export that a resolved local caller still imports; a Rust public function whose parameter count changes while a proven module-qualified caller keeps the old argument count); algorithmic changes (deeper nesting, a new nested loop, a grown function, new recursion); and taint-lite reachability (HTTP request or process arguments reaching SQL, exec, filesystem-write, or outbound-network sinks in a changed function). Signals are evidence, not verdicts; explain one with repopilot_explain_review_signal. RepoPilot uploads nothing. Only a non-empty `verify` array runs commands: the configured local checks, which may modify workspace files or contact external systems. Their captured output is bounded and redacted.",
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
                "config": { "type": "string", "description": "Optional repopilot.toml path. Defaults to the one discovered in the repository." },
                "intent_path": {
                    "type": "string",
                    "description": "Optional repository-rooted TOML file that states what the change is meant to touch. The report marks drift outside it. Use either this or `intent`, not both."
                },
                "intent": {
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
                },
                "baseline": {
                    "type": "string",
                    "description": "Optional baseline file (from `repopilot baseline create`). Findings recorded in it count as accepted debt, separate from new findings."
                },
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
                    "description": "IDs of checks configured in repopilot.toml to run on the reviewed revision. They run as local processes. Omit to run nothing."
                },
                "filters": {
                    "type": "object",
                    "description": "Optional thresholds and rule IDs that narrow the returned findings.",
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
            "readOnlyHint": false,
            "destructiveHint": true,
            "idempotentHint": false,
            "openWorldHint": true
        }
    })
}
