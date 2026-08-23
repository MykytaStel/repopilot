use super::analysis_store::{self, AnalysisKind};
use super::{ServerState, context, review_change, review_projection, scan};
use serde_json::{Value, json};

pub(super) struct PreparedToolResult {
    pub result: Value,
    publication: Option<AnalysisPublication>,
}

struct AnalysisPublication {
    kind: AnalysisKind,
    full_report: String,
    client_report: String,
    workspace_revision: String,
}

impl PreparedToolResult {
    pub(super) fn publish(self, state: &mut ServerState) -> Value {
        let Some(publication) = self.publication else {
            return self.result;
        };
        state.analyses.publish(
            publication.kind,
            publication.full_report,
            &publication.workspace_revision,
        );
        match publication.kind {
            AnalysisKind::Scan => state.last_scan = Some(publication.client_report),
            AnalysisKind::Review => state.last_review = Some(publication.client_report),
        }
        self.result
    }
}

pub(super) fn prepare_tool_result(
    name: &str,
    outcome: Result<String, String>,
    arguments: &Value,
    state: &ServerState,
    workspace_revision: &str,
    publishable: bool,
) -> PreparedToolResult {
    let kind = match name {
        scan::TOOL_NAME => Some(AnalysisKind::Scan),
        review_change::TOOL_NAME => Some(AnalysisKind::Review),
        _ => None,
    };
    let Some(kind) = kind else {
        return PreparedToolResult {
            result: tool_result(
                name,
                outcome,
                workspace_revision,
                None,
                None,
                state.max_response_bytes,
            ),
            publication: None,
        };
    };
    let full_report = match outcome {
        Ok(report) => report,
        Err(message) => {
            return error_result(name, message, workspace_revision, state.max_response_bytes);
        }
    };
    let client_report = match compact_review_for_client(kind, full_report.clone(), arguments) {
        Ok(report) => report,
        Err(message) => {
            return error_result(name, message, workspace_revision, state.max_response_bytes);
        }
    };
    let page = match analysis_store::paginate_findings(&client_report, arguments) {
        Ok(page) => page,
        Err(message) => {
            return error_result(name, message, workspace_revision, state.max_response_bytes);
        }
    };
    let handle = publishable.then(|| {
        state
            .analyses
            .prospective_handle(kind, &full_report, workspace_revision)
    });
    let result = tool_result(
        name,
        Ok(page.text.clone()),
        workspace_revision,
        handle.as_deref(),
        page.metadata,
        state.max_response_bytes,
    );
    if result["isError"] == true {
        return PreparedToolResult {
            result,
            publication: None,
        };
    }
    PreparedToolResult {
        result,
        publication: publishable.then_some(AnalysisPublication {
            kind,
            full_report,
            client_report: page.text,
            workspace_revision: workspace_revision.to_string(),
        }),
    }
}

fn error_result(
    name: &str,
    message: String,
    workspace_revision: &str,
    max_response_bytes: usize,
) -> PreparedToolResult {
    PreparedToolResult {
        result: tool_result(
            name,
            Err(message),
            workspace_revision,
            None,
            None,
            max_response_bytes,
        ),
        publication: None,
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
        review_projection::compact_review_json(&full_report)
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
    oversized_result(workspace_revision, max_response_bytes)
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

fn oversized_result(workspace_revision: &str, max_response_bytes: usize) -> Value {
    json!({
        "content": [{
            "type": "text",
            "text": format!(
                "MCP tool result exceeded the configured {max_response_bytes}-byte limit; use filters, offset/limit, detail=compact, or a smaller context budget"
            )
        }],
        "isError": true,
        "workspaceRevision": workspace_revision,
        "responseTruncated": true,
        "responseLimitBytes": max_response_bytes
    })
}
