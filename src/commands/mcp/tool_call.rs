use super::analysis_store::{self, AnalysisKind, AnalysisRecord};
use super::{
    ServerState, context, explain_file, explain_finding, explain_review_signal, review_change, scan,
};
use crate::commands::mcp::jsonrpc::Response;
use repopilot::scan::session::WorkspaceRevision;
use serde_json::{Value, json};
use std::path::Path;

pub(super) fn handle_tools_call(id: Value, params: &Value, state: &mut ServerState) -> Response {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let mut arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let current_revision = WorkspaceRevision::capture(&state.root).id().to_string();
    if let Err(message) = resolve_tool_paths(&mut arguments, &state.root) {
        return tool_response(id, name, Err(message), &current_revision, state);
    }

    let referenced = match referenced_analysis(name, &arguments, state, &current_revision) {
        Ok(record) => record,
        Err(message) => return tool_response(id, name, Err(message), &current_revision, state),
    };
    let outcome = dispatch_tool(name, &arguments, state, referenced.as_ref());

    // Analysis may update RepoPilot-owned cache files. Capture the revision at
    // response time so a handle never becomes stale because of its own call.
    let workspace_revision = WorkspaceRevision::capture(&state.root).id().to_string();
    let (outcome, analysis_handle, pagination) =
        prepare_analysis_result(name, outcome, &arguments, state, &workspace_revision);
    Response::success(
        id,
        tool_result(
            name,
            outcome,
            &workspace_revision,
            analysis_handle.as_deref(),
            pagination,
            state.max_response_bytes,
        ),
    )
}

fn tool_response(
    id: Value,
    name: &str,
    outcome: Result<String, String>,
    workspace_revision: &str,
    state: &ServerState,
) -> Response {
    Response::success(
        id,
        tool_result(
            name,
            outcome,
            workspace_revision,
            None,
            None,
            state.max_response_bytes,
        ),
    )
}

fn dispatch_tool(
    name: &str,
    arguments: &Value,
    state: &ServerState,
    referenced: Option<&AnalysisRecord>,
) -> Result<String, String> {
    match name {
        review_change::TOOL_NAME => {
            let mut full_arguments = arguments.clone();
            full_arguments["detail"] = json!("full");
            review_change::call(&full_arguments)
        }
        scan::TOOL_NAME => scan::call(arguments),
        context::TOOL_NAME => context::call(arguments),
        explain_file::TOOL_NAME => explain_file::call(arguments, &state.root),
        explain_finding::TOOL_NAME => call_explain_finding(arguments, state, referenced),
        explain_review_signal::TOOL_NAME => {
            let report = referenced
                .filter(|record| record.kind == AnalysisKind::Review)
                .map(|record| record.report.as_str())
                .or(state.last_review.as_deref());
            explain_review_signal::call(arguments, report)
        }
        other => Err(format!("unknown tool: {other}")),
    }
}

fn referenced_analysis(
    name: &str,
    arguments: &Value,
    state: &ServerState,
    workspace_revision: &str,
) -> Result<Option<AnalysisRecord>, String> {
    let Some(handle) = arguments.get("analysis_handle").and_then(Value::as_str) else {
        return Ok(None);
    };
    if !matches!(
        name,
        context::TOOL_NAME | explain_finding::TOOL_NAME | explain_review_signal::TOOL_NAME
    ) {
        return Err("`analysis_handle` is only accepted by context, explain_finding, and explain_review_signal".into());
    }
    let record = state
        .analyses
        .get(handle)
        .cloned()
        .ok_or_else(|| format!("unknown or expired analysis handle: {handle}"))?;
    if record.workspace_revision != workspace_revision {
        return Err(format!(
            "analysis handle {handle} belongs to workspace revision {}; current revision is {workspace_revision}",
            record.workspace_revision
        ));
    }
    Ok(Some(record))
}

fn call_explain_finding(
    arguments: &Value,
    state: &ServerState,
    referenced: Option<&AnalysisRecord>,
) -> Result<String, String> {
    let mut arguments = arguments.clone();
    if let Some(record) = referenced {
        let source = match record.kind {
            AnalysisKind::Scan => "last-scan",
            AnalysisKind::Review => "last-review",
        };
        arguments["source"] = json!(source);
        return explain_finding::call(
            &arguments,
            &state.root,
            (record.kind == AnalysisKind::Scan).then_some(record.report.as_str()),
            (record.kind == AnalysisKind::Review).then_some(record.report.as_str()),
        );
    }
    explain_finding::call(
        &arguments,
        &state.root,
        state.last_scan.as_deref(),
        state.last_review.as_deref(),
    )
}

fn prepare_analysis_result(
    name: &str,
    outcome: Result<String, String>,
    arguments: &Value,
    state: &mut ServerState,
    workspace_revision: &str,
) -> (Result<String, String>, Option<String>, Option<Value>) {
    let kind = match name {
        scan::TOOL_NAME => Some(AnalysisKind::Scan),
        review_change::TOOL_NAME => Some(AnalysisKind::Review),
        _ => None,
    };
    let Some(kind) = kind else {
        return (outcome, None, None);
    };
    let full_report = match outcome {
        Ok(report) => report,
        Err(message) => return (Err(message), None, None),
    };
    let handle = state
        .analyses
        .insert(kind, full_report.clone(), workspace_revision);
    let client_report = compact_review_for_client(kind, full_report, arguments);
    let client_report = match client_report {
        Ok(report) => report,
        Err(message) => return (Err(message), Some(handle), None),
    };
    match analysis_store::paginate_findings(&client_report, arguments) {
        Ok(page) => {
            match kind {
                AnalysisKind::Scan => state.last_scan = Some(page.text.clone()),
                AnalysisKind::Review => state.last_review = Some(page.text.clone()),
            }
            (Ok(page.text), Some(handle), page.metadata)
        }
        Err(message) => (Err(message), Some(handle), None),
    }
}

fn compact_review_for_client(
    kind: AnalysisKind,
    full_report: String,
    arguments: &Value,
) -> Result<String, String> {
    if kind == AnalysisKind::Review
        && arguments.get("offset").is_none()
        && arguments.get("limit").is_none()
        && arguments.get("detail").and_then(Value::as_str) != Some("full")
    {
        review_change::compact_review_json(&full_report)
    } else {
        Ok(full_report)
    }
}

pub(super) fn tool_result(
    name: &str,
    outcome: Result<String, String>,
    workspace_revision: &str,
    analysis_handle: Option<&str>,
    pagination: Option<Value>,
    max_response_bytes: usize,
) -> Value {
    let mut result = match outcome {
        Ok(text) => success_result(name, text),
        Err(message) => {
            json!({ "content": [{ "type": "text", "text": message }], "isError": true })
        }
    };
    result["workspaceRevision"] = json!(workspace_revision);
    if let Some(handle) = analysis_handle {
        result["analysisHandle"] = json!(handle);
    }
    if let Some(pagination) = pagination {
        result["pagination"] = pagination;
    }
    if serde_json::to_vec(&result).is_ok_and(|encoded| encoded.len() <= max_response_bytes) {
        return result;
    }
    oversized_result(workspace_revision, analysis_handle, max_response_bytes)
}

fn success_result(name: &str, text: String) -> Value {
    let structured = serde_json::from_str::<Value>(&text).unwrap_or_else(|_| {
        if name == context::TOOL_NAME {
            json!({ "markdown": text })
        } else {
            json!({ "text": text })
        }
    });
    json!({
        "content": [{ "type": "text", "text": text }],
        "structuredContent": structured,
        "isError": false
    })
}

fn oversized_result(
    workspace_revision: &str,
    analysis_handle: Option<&str>,
    max_response_bytes: usize,
) -> Value {
    json!({
        "content": [{
            "type": "text",
            "text": format!(
                "MCP tool result exceeded the configured {max_response_bytes}-byte limit; use filters, offset/limit, detail=compact, or a smaller context budget"
            )
        }],
        "isError": true,
        "workspaceRevision": workspace_revision,
        "analysisHandle": analysis_handle,
        "responseTruncated": true,
        "responseLimitBytes": max_response_bytes
    })
}

fn resolve_tool_paths(arguments: &mut Value, root: &Path) -> Result<(), String> {
    let confinement = repopilot::path_security::RootConfinement::named(root, "MCP root")?;
    for key in ["path", "config", "baseline"] {
        let Some(value) = arguments.get(key).and_then(Value::as_str) else {
            continue;
        };
        let resolved = confinement.resolve_allow_missing(Path::new(value), &format!("`{key}`"))?;
        arguments[key] = Value::String(resolved.to_string_lossy().to_string());
    }
    Ok(())
}
