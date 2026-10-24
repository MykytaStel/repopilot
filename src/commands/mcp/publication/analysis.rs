use super::super::analysis_store::{self, AnalysisKind};
use super::super::human_output;
use super::{
    AnalysisPublication, AnalysisRequest, PreparedToolResult, ToolResultMetadata,
    compact_review_for_client, error_result, tool_result,
};
use serde_json::Value;

struct AnalysisPage {
    text: String,
    report: Value,
    pagination: Option<Value>,
    display: String,
}

pub(super) fn prepare_analysis_result(request: AnalysisRequest<'_>) -> PreparedToolResult {
    let AnalysisRequest {
        name,
        kind,
        outcome,
        arguments,
        state,
        workspace_revision,
        publishable,
        structured_content,
    } = request;
    let full_report = match outcome {
        Ok(report) => report,
        Err(message) => {
            return error_result(name, message, workspace_revision, state.max_response_bytes);
        }
    };
    let page = match prepare_page(kind, &full_report, arguments) {
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
    let (result, client_report) = build_response(
        name,
        page,
        workspace_revision,
        handle.as_deref(),
        state.max_response_bytes,
        structured_content,
    );
    finalize_analysis_result(
        kind,
        full_report,
        workspace_revision,
        publishable,
        result,
        client_report,
    )
}

fn finalize_analysis_result(
    kind: AnalysisKind,
    full_report: String,
    workspace_revision: &str,
    publishable: bool,
    result: Value,
    client_report: String,
) -> PreparedToolResult {
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
            client_report,
            workspace_revision: workspace_revision.to_string(),
        }),
    }
}

fn prepare_page(
    kind: AnalysisKind,
    full_report: &str,
    arguments: &Value,
) -> Result<AnalysisPage, String> {
    let client_report = compact_review_for_client(kind, full_report.to_string(), arguments)?;
    let page = analysis_store::paginate_findings(&client_report, arguments)?;
    let report = serde_json::from_str::<Value>(&page.text)
        .map_err(|error| format!("MCP report could not be prepared: {error}"))?;
    let display = human_output::render(kind, &report, page.metadata.as_ref());
    Ok(AnalysisPage {
        text: page.text,
        report,
        pagination: page.metadata,
        display,
    })
}

fn build_response(
    name: &str,
    page: AnalysisPage,
    workspace_revision: &str,
    handle: Option<&str>,
    max_response_bytes: usize,
    structured_content: Option<Value>,
) -> (Value, String) {
    let client_report = page.text.clone();
    let result = tool_result(
        name,
        Ok(page.text),
        ToolResultMetadata {
            workspace_revision,
            analysis_handle: handle,
            pagination: page.pagination,
            max_response_bytes,
            structured_content: structured_content.or(Some(page.report)),
            display_text: Some(page.display),
        },
    );
    (result, client_report)
}
