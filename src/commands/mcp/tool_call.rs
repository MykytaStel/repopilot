use super::analysis_store::{AnalysisKind, AnalysisRecord};
use super::publication::{prepare_tool_result, tool_result};
use super::{
    ServerState, context, explain_file, explain_finding, explain_review_signal, review_change, scan,
};
use crate::commands::mcp::jsonrpc::Response;
use crate::commands::review_verification::ReviewVerificationEvent;
use repopilot::scan::session::WorkspaceRevision;
use repopilot::verification::CancellationToken;
use serde_json::{Value, json};
use std::path::Path;

pub(super) fn handle_tools_call(id: Value, params: &Value, state: &mut ServerState) -> Response {
    handle_tools_call_with_context(id, params, state, &CancellationToken::new(), &mut |_| {})
}

pub(super) fn handle_tools_call_with_context(
    id: Value,
    params: &Value,
    state: &mut ServerState,
    cancellation: &CancellationToken,
    observer: &mut dyn FnMut(ReviewVerificationEvent),
) -> Response {
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
    let outcome = match dispatch_tool(
        name,
        &arguments,
        state,
        referenced.as_ref(),
        cancellation,
        observer,
    ) {
        Ok(outcome) => Ok(outcome),
        Err(DispatchError::Message(message)) => Err(message),
        Err(DispatchError::Cancelled) => {
            return Response::error(id, -32800, "request cancelled");
        }
    };
    if cancellation.is_cancelled() {
        return Response::error(id, -32800, "request cancelled");
    }

    // Analysis may update RepoPilot-owned cache files. Capture the revision at
    // response time so a handle never becomes stale because of its own call.
    let workspace_revision = WorkspaceRevision::capture(&state.root).id().to_string();
    let (outcome, publishable) = match outcome {
        Ok(execution) => {
            let publishable = execution.review_revision.as_ref().is_none_or(|revision| {
                revision.revision_compatible && revision.analysis_revision == workspace_revision
            });
            (Ok(execution.rendered), publishable)
        }
        Err(message) => (Err(message), false),
    };
    let prepared = prepare_tool_result(
        name,
        outcome,
        &arguments,
        state,
        &workspace_revision,
        publishable,
    );
    if cancellation.is_cancelled() {
        return Response::error(id, -32800, "request cancelled");
    }
    Response::success(id, prepared.publish(state))
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
    cancellation: &CancellationToken,
    observer: &mut dyn FnMut(ReviewVerificationEvent),
) -> Result<ToolExecution, DispatchError> {
    match name {
        review_change::TOOL_NAME => {
            let mut full_arguments = arguments.clone();
            full_arguments["detail"] = json!("full");
            review_change::call_with_context(
                &full_arguments,
                review_change::ReviewCallContext {
                    cancellation,
                    observer,
                },
            )
            .map(|result| ToolExecution {
                rendered: result.rendered,
                review_revision: Some(ReviewRevision {
                    analysis_revision: result.analysis_revision,
                    revision_compatible: result.revision_compatible,
                }),
            })
            .map_err(|error| match error {
                review_change::ReviewCallError::Cancelled => DispatchError::Cancelled,
                review_change::ReviewCallError::Message(message) => DispatchError::Message(message),
            })
        }
        scan::TOOL_NAME => scan::call(arguments)
            .map(ToolExecution::plain)
            .map_err(DispatchError::Message),
        context::TOOL_NAME => context::call(arguments)
            .map(ToolExecution::plain)
            .map_err(DispatchError::Message),
        explain_file::TOOL_NAME => explain_file::call(arguments, &state.root)
            .map(ToolExecution::plain)
            .map_err(DispatchError::Message),
        explain_finding::TOOL_NAME => call_explain_finding(arguments, state, referenced)
            .map(ToolExecution::plain)
            .map_err(DispatchError::Message),
        explain_review_signal::TOOL_NAME => {
            let report = referenced
                .filter(|record| record.kind == AnalysisKind::Review)
                .map(|record| record.report.as_str())
                .or(state.last_review.as_deref());
            explain_review_signal::call(arguments, report)
                .map(ToolExecution::plain)
                .map_err(DispatchError::Message)
        }
        other => Err(DispatchError::Message(format!("unknown tool: {other}"))),
    }
}

enum DispatchError {
    Cancelled,
    Message(String),
}

struct ToolExecution {
    rendered: String,
    review_revision: Option<ReviewRevision>,
}

impl ToolExecution {
    fn plain(rendered: String) -> Self {
        Self {
            rendered,
            review_revision: None,
        }
    }
}

struct ReviewRevision {
    analysis_revision: String,
    revision_compatible: bool,
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
