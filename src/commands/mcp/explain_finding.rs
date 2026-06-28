//! The `repopilot_explain_finding` MCP tool: replay one emitted finding by
//! stable ID from the latest scan or review stored in this MCP session.

use repopilot::explain::build_finding_explanation_from_report;
use serde_json::{Value, json};
use std::path::Path;

pub const TOOL_NAME: &str = "repopilot_explain_finding";

pub fn definition() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": "Explain an emitted finding by stable ID using the latest scan or review in this MCP session. Replays the stored Knowledge Engine inputs against the current workspace, returns the full decision trace, and reports matched vs drifted decisions. Local-only.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "finding_id": {
                    "type": "string",
                    "description": "Stable finding ID from the findings array of the selected session report."
                },
                "source": {
                    "type": "string",
                    "enum": ["last-scan", "last-review"],
                    "default": "last-scan",
                    "description": "Session report containing the finding."
                }
            },
            "required": ["finding_id"],
            "additionalProperties": false
        },
        "outputSchema": { "type": "object", "additionalProperties": true },
        "annotations": {
            "readOnlyHint": true,
            "destructiveHint": false,
            "idempotentHint": true,
            "openWorldHint": false
        }
    })
}

pub fn call(
    arguments: &Value,
    root: &Path,
    last_scan: Option<&str>,
    last_review: Option<&str>,
) -> Result<String, String> {
    let finding_id = arguments
        .get("finding_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "`finding_id` is required".to_string())?;
    let source = arguments
        .get("source")
        .and_then(Value::as_str)
        .unwrap_or("last-scan");

    let report = match source {
        "last-scan" => last_scan.ok_or_else(|| {
            "no scan is available in this MCP session; run repopilot_scan first".to_string()
        })?,
        "last-review" => last_review.ok_or_else(|| {
            "no review is available in this MCP session; run repopilot_review_change first"
                .to_string()
        })?,
        other => return Err(format!("invalid source: {other}")),
    };

    let explanation = build_finding_explanation_from_report(root, report, finding_id, source)?;
    serde_json::to_string_pretty(&explanation)
        .map_err(|error| format!("render finding explanation failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use repopilot::explain::build_explain_report_with_root;
    use repopilot::findings::provenance::{
        AnalysisScope, FindingProvenance, KnowledgeDecisionAction, KnowledgeDecisionProvenance,
    };
    use repopilot::findings::types::{Confidence, Evidence, Finding, FindingCategory, Severity};
    use repopilot::rules::{RuleLifecycle, SignalSource};
    use std::path::PathBuf;

    fn report_fixture(root: &Path) -> (String, String) {
        let path = root.join("src/domain/service.rs");
        std::fs::create_dir_all(path.parent().expect("parent")).expect("create src");
        std::fs::write(&path, "pub fn run() { panic!(\"boom\"); }\n").expect("write file");

        let base_severity = Severity::Medium;
        let signal = "rust.panic";
        let source_explanation = build_explain_report_with_root(
            root,
            &path,
            Some("language.rust.panic-risk"),
            Some(signal),
            base_severity,
        )
        .expect("build explanation");
        let decision = source_explanation.decision.expect("decision");
        let action = match decision.action.as_str() {
            "apply" => KnowledgeDecisionAction::Apply,
            "suppress" => KnowledgeDecisionAction::Suppress,
            "downgrade" => KnowledgeDecisionAction::Downgrade,
            "upgrade" => KnowledgeDecisionAction::Upgrade,
            other => panic!("unexpected action: {other}"),
        };
        let finding_id = "language.rust.panic-risk:src/domain/service.rs:1".to_string();
        let finding = Finding {
            id: finding_id.clone(),
            rule_id: "language.rust.panic-risk".to_string(),
            title: "Panic risk".to_string(),
            description: "panic can terminate the current operation".to_string(),
            recommendation: "Return a typed error where possible.".to_string(),
            category: FindingCategory::CodeQuality,
            severity: decision.final_severity,
            confidence: Confidence::High,
            evidence: vec![Evidence {
                path: PathBuf::from("src/domain/service.rs"),
                line_start: 1,
                line_end: None,
                snippet: "panic!(\"boom\")".to_string(),
            }],
            workspace_package: None,
            docs_url: None,
            provenance: FindingProvenance {
                detector: "language.rust.panic-risk".to_string(),
                signal_source: SignalSource::Ast,
                rule_lifecycle: RuleLifecycle::Stable,
                analysis_scope: AnalysisScope::File,
                knowledge_decision: Some(KnowledgeDecisionProvenance {
                    base_severity,
                    signal: Some(signal.to_string()),
                    action,
                    decided_severity: decision.final_severity,
                    reason: decision.reason,
                }),
            },
            risk: Default::default(),
        };

        (
            serde_json::to_string(&json!({
                "schema_version": "0.20",
                "root_path": root,
                "findings": [finding]
            }))
            .expect("serialize report"),
            finding_id,
        )
    }

    #[test]
    fn tool_replays_a_finding_from_last_scan() {
        let temp = tempfile::tempdir().expect("tempdir");
        let (report, finding_id) = report_fixture(temp.path());

        let rendered = call(
            &json!({
                "finding_id": finding_id,
                "source": "last-scan"
            }),
            temp.path(),
            Some(&report),
            None,
        )
        .expect("explain finding");

        let value: Value = serde_json::from_str(&rendered).expect("valid JSON");
        assert_eq!(value["source_report"], "last-scan");
        assert_eq!(value["replay"]["status"], "matched");
        assert!(value["explanation"]["decision"]["trace"].is_array());
    }

    #[test]
    fn tool_requires_the_selected_session_report() {
        let error = call(
            &json!({ "finding_id": "missing" }),
            Path::new("."),
            None,
            None,
        )
        .expect_err("missing scan must fail");

        assert!(error.contains("run repopilot_scan first"));
    }
}
