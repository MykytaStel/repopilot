mod analysis;

use super::analysis_store::AnalysisKind;
use super::{ServerState, context, human_output, review_change, review_projection, scan};
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

pub(super) struct AnalysisRequest<'a> {
    name: &'a str,
    kind: AnalysisKind,
    outcome: Result<String, String>,
    arguments: &'a Value,
    state: &'a ServerState,
    workspace_revision: &'a str,
    publishable: bool,
    structured_content: Option<Value>,
}

pub(super) struct ToolResultMetadata<'a> {
    pub(super) workspace_revision: &'a str,
    pub(super) analysis_handle: Option<&'a str>,
    pub(super) pagination: Option<Value>,
    pub(super) max_response_bytes: usize,
    pub(super) structured_content: Option<Value>,
    pub(super) display_text: Option<String>,
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
    structured_content: Option<Value>,
) -> PreparedToolResult {
    let kind = analysis_kind(name);
    match kind {
        Some(kind) => analysis::prepare_analysis_result(AnalysisRequest {
            name,
            kind,
            outcome,
            arguments,
            state,
            workspace_revision,
            publishable,
            structured_content,
        }),
        None => prepare_non_analysis_result(
            name,
            outcome,
            state,
            workspace_revision,
            structured_content,
        ),
    }
}

fn analysis_kind(name: &str) -> Option<AnalysisKind> {
    match name {
        scan::TOOL_NAME => Some(AnalysisKind::Scan),
        review_change::TOOL_NAME => Some(AnalysisKind::Review),
        _ => None,
    }
}

fn prepare_non_analysis_result(
    name: &str,
    outcome: Result<String, String>,
    state: &ServerState,
    workspace_revision: &str,
    structured_content: Option<Value>,
) -> PreparedToolResult {
    let (outcome, structured_content, display_text) = match outcome {
        Ok(text) => {
            let parsed = serde_json::from_str::<Value>(&text).ok();
            let display = parsed
                .as_ref()
                .and_then(|report| human_output::render_explanation(name, report));
            (Ok(text), structured_content.or(parsed), display)
        }
        Err(message) => (Err(message), structured_content, None),
    };
    PreparedToolResult {
        result: tool_result(
            name,
            outcome,
            ToolResultMetadata {
                workspace_revision,
                analysis_handle: None,
                pagination: None,
                max_response_bytes: state.max_response_bytes,
                structured_content,
                display_text,
            },
        ),
        publication: None,
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
            ToolResultMetadata {
                workspace_revision,
                analysis_handle: None,
                pagination: None,
                max_response_bytes,
                structured_content: None,
                display_text: None,
            },
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
    metadata: ToolResultMetadata<'_>,
) -> Value {
    let mut result = match outcome {
        Ok(text) => success_result(
            name,
            text,
            metadata.structured_content,
            metadata.display_text,
        ),
        Err(message) => {
            json!({ "content": [{ "type": "text", "text": message }], "isError": true })
        }
    };
    result["workspaceRevision"] = json!(metadata.workspace_revision);
    if let Some(handle) = metadata.analysis_handle {
        result["analysisHandle"] = json!(handle);
    }
    if let Some(pagination) = metadata.pagination {
        result["pagination"] = pagination;
    }
    if serde_json::to_vec(&result).is_ok_and(|encoded| encoded.len() <= metadata.max_response_bytes)
    {
        return result;
    }
    oversized_result(metadata.workspace_revision, metadata.max_response_bytes)
}

fn success_result(
    name: &str,
    text: String,
    structured_content: Option<Value>,
    display_text: Option<String>,
) -> Value {
    let structured = structured_content.unwrap_or_else(|| {
        serde_json::from_str::<Value>(&text).unwrap_or_else(|_| {
            if name == context::TOOL_NAME {
                json!({ "markdown": text })
            } else {
                json!({ "text": text })
            }
        })
    });
    json!({
        "content": [{ "type": "text", "text": display_text.unwrap_or(text) }],
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
