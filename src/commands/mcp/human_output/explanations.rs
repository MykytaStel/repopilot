use serde_json::Value;

pub fn render_explanation(tool: &str, report: &Value) -> Option<String> {
    match tool {
        super::super::explain_file::TOOL_NAME => Some(super::file::render_file(report)),
        super::super::explain_finding::TOOL_NAME => Some(super::finding::render_finding(report)),
        super::super::explain_review_signal::TOOL_NAME => {
            Some(super::signal::render_signal(report))
        }
        _ => None,
    }
}
