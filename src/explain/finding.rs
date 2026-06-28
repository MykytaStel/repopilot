use super::builder::build_explain_report_with_root;
use super::model::ExplainReport;
use crate::findings::provenance::{
    AnalysisScope, KnowledgeDecisionAction, KnowledgeDecisionProvenance,
};
use crate::findings::types::Finding;
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FindingExplanationReport {
    pub source_report: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_schema_version: Option<String>,
    pub source_root: String,
    pub finding: Finding,
    pub stored_decision: KnowledgeDecisionProvenance,
    pub replay: FindingDecisionReplay,
    pub explanation: ExplainReport,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FindingDecisionReplay {
    pub status: FindingReplayStatus,
    pub action_matches: bool,
    pub severity_matches: bool,
    pub reason_matches: bool,
    pub detector_local_severity_adjustment: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum FindingReplayStatus {
    Matched,
    Drifted,
}

pub fn build_finding_explanation_from_report(
    mcp_root: &Path,
    report_json: &str,
    finding_id: &str,
    source_report: &str,
) -> Result<FindingExplanationReport, String> {
    if finding_id.trim().is_empty() {
        return Err("`finding_id` must not be empty".to_string());
    }

    let report: Value = serde_json::from_str(report_json)
        .map_err(|error| format!("invalid session report: {error}"))?;
    let findings = report
        .get("findings")
        .and_then(Value::as_array)
        .ok_or_else(|| "session report does not contain a findings array".to_string())?;

    let matches = findings
        .iter()
        .filter(|finding| finding.get("id").and_then(Value::as_str) == Some(finding_id))
        .collect::<Vec<_>>();

    let finding_value = match matches.as_slice() {
        [] => {
            return Err(format!(
                "finding `{finding_id}` was not found in {source_report}"
            ));
        }
        [finding] => (*finding).clone(),
        _ => {
            return Err(format!(
                "finding `{finding_id}` appears more than once in {source_report}"
            ));
        }
    };

    // Review JSON flattens Finding and appends in_diff/baseline_status fields.
    // Serde ignores those additive fields while preserving the complete Finding.
    let finding: Finding = serde_json::from_value(finding_value)
        .map_err(|error| format!("invalid finding `{finding_id}`: {error}"))?;

    if finding.provenance.analysis_scope != AnalysisScope::File {
        return Err(format!(
            "finding `{finding_id}` uses `{}` analysis scope;              repopilot_explain_finding currently supports file-scoped findings only",
            analysis_scope_id(finding.provenance.analysis_scope)
        ));
    }

    let stored_decision = finding
        .provenance
        .knowledge_decision
        .clone()
        .ok_or_else(|| {
            format!(
                "finding `{finding_id}` has no knowledge decision provenance; \
                 run RepoPilot schema 0.20+ and choose a knowledge-aware finding"
            )
        })?;

    let canonical_mcp_root = mcp_root
        .canonicalize()
        .unwrap_or_else(|_| mcp_root.to_path_buf());
    let source_root = resolve_source_root(&canonical_mcp_root, &report)?;
    let evidence = finding
        .evidence
        .first()
        .ok_or_else(|| format!("finding `{finding_id}` has no evidence path"))?;
    let source_path = resolve_source_path(&canonical_mcp_root, &source_root, &evidence.path)?;

    let explanation = build_explain_report_with_root(
        &source_root,
        &source_path,
        Some(&finding.rule_id),
        stored_decision.signal.as_deref(),
        stored_decision.base_severity,
    )
    .map_err(|error| format!("finding replay failed: {error}"))?;

    let replayed = explanation
        .decision
        .as_ref()
        .ok_or_else(|| "finding replay did not produce a rule decision".to_string())?;
    let stored_action = knowledge_action_id(stored_decision.action);

    let action_matches = replayed.action == stored_action;
    let severity_matches = replayed.final_severity == stored_decision.decided_severity;
    let reason_matches = replayed.reason == stored_decision.reason;
    let detector_local_severity_adjustment = finding.severity != stored_decision.decided_severity;

    let mut notes = Vec::new();
    if !action_matches {
        notes.push(format!(
            "decision action changed: stored={} replayed={}",
            stored_action, replayed.action
        ));
    }
    if !severity_matches {
        notes.push(format!(
            "decision severity changed: stored={} replayed={}",
            stored_decision.decided_severity.label(),
            replayed.final_severity.label()
        ));
    }
    if !reason_matches {
        notes.push("decision reason changed".to_string());
    }
    if detector_local_severity_adjustment {
        notes.push(format!(
            "detector-local policy changed the emitted finding severity from {} to {}",
            stored_decision.decided_severity.label(),
            finding.severity.label()
        ));
    }

    let status = if action_matches && severity_matches && reason_matches {
        FindingReplayStatus::Matched
    } else {
        FindingReplayStatus::Drifted
    };

    Ok(FindingExplanationReport {
        source_report: source_report.to_string(),
        source_schema_version: report
            .get("schema_version")
            .and_then(Value::as_str)
            .map(str::to_string),
        source_root: source_root.to_string_lossy().to_string(),
        finding,
        stored_decision,
        replay: FindingDecisionReplay {
            status,
            action_matches,
            severity_matches,
            reason_matches,
            detector_local_severity_adjustment,
            notes,
        },
        explanation,
    })
}

fn resolve_source_root(mcp_root: &Path, report: &Value) -> Result<PathBuf, String> {
    let value = report
        .get("root_path")
        .and_then(Value::as_str)
        .unwrap_or(".");
    let candidate = if Path::new(value).is_absolute() {
        PathBuf::from(value)
    } else {
        mcp_root.join(value)
    };
    let resolved = candidate.canonicalize().unwrap_or(candidate);
    ensure_within_mcp_root(mcp_root, &resolved, "session report root")?;
    Ok(resolved)
}

fn resolve_source_path(
    mcp_root: &Path,
    source_root: &Path,
    evidence_path: &Path,
) -> Result<PathBuf, String> {
    let candidate = if evidence_path.is_absolute() {
        evidence_path.to_path_buf()
    } else {
        source_root.join(evidence_path)
    };
    let resolved = candidate.canonicalize().map_err(|error| {
        format!(
            "finding evidence file {} is unavailable: {error}",
            candidate.display()
        )
    })?;
    ensure_within_mcp_root(mcp_root, &resolved, "finding evidence path")?;
    Ok(resolved)
}

fn ensure_within_mcp_root(root: &Path, path: &Path, label: &str) -> Result<(), String> {
    if path.starts_with(root) {
        Ok(())
    } else {
        Err(format!(
            "{label} {} must stay within MCP root {}",
            path.display(),
            root.display()
        ))
    }
}

fn analysis_scope_id(scope: AnalysisScope) -> &'static str {
    match scope {
        AnalysisScope::File => "file",
        AnalysisScope::Repository => "repository",
        AnalysisScope::Workspace => "workspace",
        AnalysisScope::GitDiff => "git-diff",
        AnalysisScope::FrameworkProject => "framework-project",
    }
}

fn knowledge_action_id(action: KnowledgeDecisionAction) -> &'static str {
    match action {
        KnowledgeDecisionAction::Apply => "apply",
        KnowledgeDecisionAction::Suppress => "suppress",
        KnowledgeDecisionAction::Downgrade => "downgrade",
        KnowledgeDecisionAction::Upgrade => "upgrade",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::findings::provenance::{
        AnalysisScope, FindingProvenance, KnowledgeDecisionAction, KnowledgeDecisionProvenance,
    };
    use crate::findings::types::{Confidence, Evidence, FindingCategory, Severity};
    use crate::rules::{RuleLifecycle, SignalSource};
    use serde_json::json;

    fn fixture_report(root: &Path, stored_reason: Option<String>) -> (String, String) {
        let path = root.join("src/domain/service.rs");
        std::fs::create_dir_all(path.parent().expect("parent")).expect("create src");
        std::fs::write(&path, "pub fn run() { panic!(\"boom\"); }\n").expect("write fixture");

        let base_severity = Severity::Medium;
        let signal = "rust.panic";
        let explanation = build_explain_report_with_root(
            root,
            &path,
            Some("language.rust.panic-risk"),
            Some(signal),
            base_severity,
        )
        .expect("build source explanation");
        let decision = explanation.decision.expect("decision");
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
            recommendation: "Return a typed error where recovery is possible.".to_string(),
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
                    reason: stored_reason.or(decision.reason),
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
    fn replays_a_finding_from_a_session_report() {
        let temp = tempfile::tempdir().expect("tempdir");
        let (report, finding_id) = fixture_report(temp.path(), None);

        let replay =
            build_finding_explanation_from_report(temp.path(), &report, &finding_id, "last-scan")
                .expect("replay finding");

        assert_eq!(replay.replay.status, FindingReplayStatus::Matched);
        assert!(replay.replay.action_matches);
        assert!(replay.replay.severity_matches);
        assert!(replay.replay.reason_matches);
        assert_eq!(replay.finding.id, finding_id);
        assert!(
            replay
                .explanation
                .decision
                .as_ref()
                .is_some_and(|decision| !decision.trace.is_empty())
        );
    }

    #[test]
    fn reports_decision_drift_without_hiding_the_trace() {
        let temp = tempfile::tempdir().expect("tempdir");
        let (report, finding_id) =
            fixture_report(temp.path(), Some("historical reason".to_string()));

        let replay =
            build_finding_explanation_from_report(temp.path(), &report, &finding_id, "last-review")
                .expect("replay finding");

        assert_eq!(replay.replay.status, FindingReplayStatus::Drifted);
        assert!(!replay.replay.reason_matches);
        assert!(
            replay
                .replay
                .notes
                .iter()
                .any(|note| note == "decision reason changed")
        );
        assert!(replay.explanation.decision.is_some());
    }

    #[test]
    fn rejects_findings_without_knowledge_decision_provenance() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("src/lib.rs");
        std::fs::create_dir_all(path.parent().expect("parent")).expect("create src");
        std::fs::write(&path, "pub fn live() {}\n").expect("write file");

        let finding = Finding {
            id: "code-marker.todo:src/lib.rs:1".to_string(),
            rule_id: "code-marker.todo".to_string(),
            title: "TODO".to_string(),
            description: "TODO marker".to_string(),
            category: FindingCategory::CodeQuality,
            severity: Severity::Low,
            evidence: vec![Evidence {
                path: PathBuf::from("src/lib.rs"),
                line_start: 1,
                line_end: None,
                snippet: "TODO".to_string(),
            }],
            ..Finding::default()
        };
        let report = serde_json::to_string(&json!({
            "schema_version": "0.20",
            "root_path": temp.path(),
            "findings": [finding]
        }))
        .expect("serialize report");

        let error = build_finding_explanation_from_report(
            temp.path(),
            &report,
            "code-marker.todo:src/lib.rs:1",
            "last-scan",
        )
        .expect_err("missing knowledge provenance must fail");

        assert!(error.contains("has no knowledge decision provenance"));
    }

    #[test]
    fn rejects_repository_scoped_findings_instead_of_file_replay() {
        let temp = tempfile::tempdir().expect("tempdir");
        let (report, finding_id) = fixture_report(temp.path(), None);
        let mut value: Value = serde_json::from_str(&report).expect("report JSON");
        value["findings"][0]["provenance"]["analysis_scope"] = json!("repository");
        let report = serde_json::to_string(&value).expect("serialize report");

        let error =
            build_finding_explanation_from_report(temp.path(), &report, &finding_id, "last-scan")
                .expect_err("repository-scoped replay must fail");

        assert!(error.contains("uses `repository` analysis scope"));
        assert!(error.contains("currently supports file-scoped findings only"));
    }
}
