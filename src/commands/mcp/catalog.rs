use super::{
    ServerState, context, explain_file, explain_finding, explain_review_signal, review_change, scan,
};
use crate::commands::mcp::jsonrpc::Response;
use serde_json::{Value, json};

/// Returned as `instructions` from `initialize`; clients place it ahead of the
/// tool catalog, so it routes a question to the right tool.
pub(super) const SERVER_INSTRUCTIONS: &str = "RepoPilot reviews Git changes and repositories locally and deterministically. It calls no model and uploads nothing. Pick a tool by the question:\n\
- What did this change touch, and which checks did it weaken? repopilot_review_change (uncommitted work by default; pass `base`, e.g. \"origin/main\", for a branch).\n\
- What needs attention in this repository or folder overall? repopilot_scan.\n\
- What should I know before editing this codebase? repopilot_context.\n\
- Why was this finding reported? repopilot_explain_finding with its `finding_id`, after a scan or review.\n\
- Why was this review signal raised? repopilot_explain_review_signal with its `signal_id`, after a review.\n\
- How does RepoPilot treat this file, and which rules apply to it? repopilot_explain_file.\n\
Findings and signals are evidence, not verdicts: confirm the impact in the code. Re-run a review after editing; stored results describe the workspace at the time of the call.";

pub(super) fn tools_list_result() -> Value {
    json!({
        "tools": [
            review_change::definition(),
            scan::definition(),
            context::definition(),
            explain_file::definition(),
            explain_finding::definition(),
            explain_review_signal::definition(),
        ]
    })
}

pub(super) fn resources_list_result(state: &ServerState) -> Value {
    let mut resources = vec![
        json!({
            "uri": "repopilot://rules",
            "name": "RepoPilot rule catalog",
            "mimeType": "application/json"
        }),
        json!({
            "uri": "repopilot://repository-summary",
            "name": "RepoPilot repository summary",
            "mimeType": "application/json"
        }),
        json!({
            "uri": "repopilot://analyses",
            "name": "Available RepoPilot analysis handles",
            "mimeType": "application/json"
        }),
    ];
    if state.last_scan.is_some() {
        resources.push(json!({
            "uri": "repopilot://last-scan",
            "name": "Last RepoPilot scan",
            "mimeType": "application/json"
        }));
    }
    if state.last_review.is_some() {
        resources.push(json!({
            "uri": "repopilot://last-review",
            "name": "Last RepoPilot review",
            "mimeType": "application/json"
        }));
    }
    json!({ "resources": resources })
}

pub(super) fn handle_resource_read(id: Value, params: &Value, state: &ServerState) -> Response {
    let uri = params
        .get("uri")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let text = match uri {
        "repopilot://analyses" => serde_json::to_string_pretty(&state.analyses.summaries())
            .unwrap_or_else(|_| "[]".to_string()),
        "repopilot://rules" => {
            let rules = repopilot::rules::all_rule_metadata()
                .map(|rule| {
                    json!({
                        "rule_id": rule.rule_id,
                        "title": rule.title,
                        "category": rule.category.label(),
                        "severity": rule.default_severity.label(),
                        "max_severity": rule.severity_ceiling().label(),
                        "confidence": rule.default_confidence.label(),
                        "max_confidence": rule.confidence_ceiling().label(),
                        "lifecycle": rule.lifecycle.label(),
                        "signal_source": rule.signal_source.label(),
                        "docs_url": rule.docs_url
                    })
                })
                .collect::<Vec<_>>();
            serde_json::to_string_pretty(&rules).unwrap_or_else(|_| "[]".to_string())
        }
        "repopilot://last-scan" => state.last_scan.clone().unwrap_or_default(),
        "repopilot://last-review" => state.last_review.clone().unwrap_or_default(),
        "repopilot://repository-summary" => serde_json::to_string_pretty(&json!({
            "root": state.root.to_string_lossy(),
            "git_repository": state.root.join(".git").exists(),
            "config_present": state.root.join("repopilot.toml").is_file(),
            "baseline_present": state.root.join(".repopilot/baseline.json").is_file(),
            "feedback_present": state.root.join(".repopilot/feedback.yml").is_file(),
            "last_scan_available": state.last_scan.is_some(),
            "last_review_available": state.last_review.is_some()
        }))
        .unwrap_or_else(|_| "{}".to_string()),
        _ => return Response::error(id, -32002, format!("resource not found: {uri}")),
    };
    Response::success(
        id,
        json!({ "contents": [{ "uri": uri, "mimeType": "application/json", "text": text }] }),
    )
}

pub(super) fn prompts_list_result() -> Value {
    json!({
        "prompts": [
            {
                "name": "review-change",
                "description": "Review the current change with RepoPilot evidence."
            },
            {
                "name": "fix-top-risk",
                "description": "Plan the smallest fix for the highest-priority RepoPilot risk."
            }
        ]
    })
}

pub(super) fn handle_prompt_get(id: Value, params: &Value) -> Response {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let text = match name {
        "review-change" => {
            "Call repopilot_review_change, inspect definitely-sensitive signals first, and report evidence without claiming the change is safe."
        }
        "fix-top-risk" => {
            "Call repopilot_scan, select the highest-priority evidence-backed finding, and propose the smallest verified remediation."
        }
        _ => return Response::error(id, -32602, format!("unknown prompt: {name}")),
    };
    Response::success(
        id,
        json!({
            "description": text,
            "messages": [{
                "role": "user",
                "content": { "type": "text", "text": text }
            }]
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::{SERVER_INSTRUCTIONS, tools_list_result};
    use serde_json::Value;

    fn tools() -> Vec<Value> {
        tools_list_result()["tools"]
            .as_array()
            .expect("tools array")
            .clone()
    }

    #[test]
    fn every_tool_points_to_a_sibling_and_describes_its_inputs() {
        for tool in tools() {
            let name = tool["name"].as_str().expect("tool name");
            let description = tool["description"].as_str().expect("description");
            let names_sibling = tools().iter().any(|other| {
                let other = other["name"].as_str().unwrap_or_default();
                other != name && description.contains(other)
            });
            assert!(
                names_sibling,
                "{name} should say when to use a sibling tool"
            );

            let properties = tool["inputSchema"]["properties"]
                .as_object()
                .expect("input properties");
            for (property, schema) in properties {
                let text = schema["description"].as_str().unwrap_or_default();
                assert!(!text.is_empty(), "{name}.{property} has no description");
            }
        }
    }

    #[test]
    fn server_instructions_route_to_every_tool() {
        for tool in tools() {
            let name = tool["name"].as_str().expect("tool name");
            assert!(
                SERVER_INSTRUCTIONS.contains(name),
                "instructions should mention {name}"
            );
        }
    }
}
