//! The `repopilot_explain_finding` MCP tool: replay one emitted finding by
//! stable ID from the latest scan or review stored in this MCP session.

use repopilot::explain::{
    FindingOccurrenceLocator, build_finding_explanation_selection_from_report,
};
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
                },
                "analysis_handle": {
                    "type": "string",
                    "description": "Optional scan/review handle to select a stored analysis instead of the latest result."
                },
                "evidence_path": {
                    "type": "string",
                    "description": "Optional evidence path from an ambiguity candidate. Used with line_start to select one occurrence without changing the stable finding ID."
                },
                "line_start": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional evidence start line from an ambiguity candidate."
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
    let locator = FindingOccurrenceLocator {
        evidence_path: arguments
            .get("evidence_path")
            .and_then(Value::as_str)
            .map(str::to_string),
        line_start: arguments
            .get("line_start")
            .and_then(Value::as_u64)
            .and_then(|line| usize::try_from(line).ok()),
    };
    let locator = (!locator.is_empty()).then_some(locator);

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

    let selection = match build_finding_explanation_selection_from_report(
        root,
        report,
        finding_id,
        source,
        locator.as_ref(),
    ) {
        Ok(selection) => selection,
        Err(reason) => {
            return stored_fallback(report, finding_id, source, locator.as_ref(), &reason);
        }
    };
    serde_json::to_string_pretty(&selection)
        .map_err(|error| format!("render finding explanation failed: {error}"))
}

fn stored_fallback(
    report: &str,
    finding_id: &str,
    source: &str,
    locator: Option<&FindingOccurrenceLocator>,
    reason: &str,
) -> Result<String, String> {
    let report: Value =
        serde_json::from_str(report).map_err(|error| format!("invalid session report: {error}"))?;
    let findings = report
        .get("findings")
        .and_then(Value::as_array)
        .ok_or_else(|| "session report does not contain a findings array".to_string())?;
    let finding = findings.iter().find(|finding| {
        finding.get("id").and_then(Value::as_str) == Some(finding_id)
            && locator.is_none_or(|locator| fallback_matches_locator(finding, locator))
    });
    let Some(finding) = finding else {
        return Err(reason.to_string());
    };
    let fallback = json!({
        "status": "stored-only",
        "source_report": source,
        "finding_id": finding_id,
        "message": "The finding is available, but a safe live decision replay is unavailable.",
        "replay": { "status": "unavailable", "reason": reason },
        "finding": finding,
        "decision": finding.get("decision").cloned().unwrap_or(Value::Null),
        "limitations": [
            "This fallback preserves stored analysis evidence and does not claim a live replay.",
            "Re-run the originating scan or review after workspace or configuration changes."
        ]
    });
    serde_json::to_string_pretty(&fallback)
        .map_err(|error| format!("render finding fallback failed: {error}"))
}

fn fallback_matches_locator(finding: &Value, locator: &FindingOccurrenceLocator) -> bool {
    let evidence = finding
        .get("evidence")
        .and_then(Value::as_array)
        .and_then(|items| items.first());
    locator.evidence_path.as_deref().is_none_or(|path| {
        evidence
            .and_then(|item| item.get("path"))
            .and_then(Value::as_str)
            == Some(path)
    }) && locator.line_start.is_none_or(|line| {
        evidence
            .and_then(|item| item.get("line_start"))
            .and_then(Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            == Some(line)
    })
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
                    overlay_applied: false,
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

    #[test]
    fn tool_returns_stored_fallback_when_live_replay_is_unsupported() {
        let report = json!({
            "findings": [{
                "id": "project-wide",
                "rule_id": "architecture.example",
                "title": "Project-wide finding",
                "description": "Stored evidence remains useful.",
                "decision": { "severity": "medium" },
                "provenance": { "analysis_scope": "project" },
                "evidence": [{ "path": "Cargo.toml", "line_start": 1 }]
            }]
        })
        .to_string();

        let rendered = call(
            &json!({ "finding_id": "project-wide" }),
            Path::new("."),
            Some(&report),
            None,
        )
        .expect("stored fallback");
        let value: Value = serde_json::from_str(&rendered).expect("valid JSON");
        assert_eq!(value["status"], "stored-only");
        assert_eq!(value["replay"]["status"], "unavailable");
        assert_eq!(value["finding"]["id"], "project-wide");
        assert_eq!(value["decision"]["severity"], "medium");
    }

    fn duplicate_report(report: &str) -> String {
        let mut value: Value = serde_json::from_str(report).expect("report JSON");
        let mut duplicate = value["findings"][0].clone();
        duplicate["evidence"][0]["line_start"] = json!(2);
        duplicate["evidence"][0]["snippet"] = json!("panic!(\"second\")");
        value["findings"]
            .as_array_mut()
            .expect("findings array")
            .push(duplicate);
        serde_json::to_string(&value).expect("serialize duplicate report")
    }

    #[test]
    fn tool_returns_candidates_for_an_ambiguous_stable_id() {
        let temp = tempfile::tempdir().expect("tempdir");
        let (report, finding_id) = report_fixture(temp.path());
        let report = duplicate_report(&report);

        let rendered = call(
            &json!({
                "finding_id": finding_id,
                "source": "last-scan"
            }),
            temp.path(),
            Some(&report),
            None,
        )
        .expect("ambiguity is structured output");

        let value: Value = serde_json::from_str(&rendered).expect("valid JSON");
        assert_eq!(value["status"], "ambiguous");
        assert_eq!(value["candidates"].as_array().map(Vec::len), Some(2));
        assert_eq!(
            value["candidates"][0]["evidence_path"],
            "src/domain/service.rs"
        );
        assert_eq!(value["candidates"][0]["line_start"], 1);
        assert_eq!(value["candidates"][1]["line_start"], 2);
    }

    #[test]
    fn tool_resolves_an_ambiguous_id_with_an_occurrence_locator() {
        let temp = tempfile::tempdir().expect("tempdir");
        let (report, finding_id) = report_fixture(temp.path());
        let report = duplicate_report(&report);

        let rendered = call(
            &json!({
                "finding_id": finding_id,
                "source": "last-scan",
                "evidence_path": "src/domain/service.rs",
                "line_start": 2
            }),
            temp.path(),
            Some(&report),
            None,
        )
        .expect("resolve finding occurrence");

        let value: Value = serde_json::from_str(&rendered).expect("valid JSON");
        assert_eq!(value["finding"]["evidence"][0]["line_start"], 2);
        assert_eq!(value["replay"]["status"], "matched");
        assert!(value.get("status").is_none());
    }
}
