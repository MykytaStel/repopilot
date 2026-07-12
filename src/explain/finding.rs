use super::builder::build_explain_report_with_root;
use super::model::ExplainReport;
use crate::findings::decision::{DecisionRecord, build_decision_record};
use crate::findings::occurrence::occurrence_key;
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
    pub occurrence_key: String,
    pub decision: DecisionRecord,
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

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct FindingOccurrenceLocator {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_start: Option<usize>,
}

impl FindingOccurrenceLocator {
    pub fn is_empty(&self) -> bool {
        self.evidence_path.is_none() && self.line_start.is_none()
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FindingOccurrenceCandidate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_start: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_end: Option<usize>,
    pub rule_id: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FindingAmbiguityReport {
    pub status: String,
    pub source_report: String,
    pub finding_id: String,
    pub message: String,
    pub candidates: Vec<FindingOccurrenceCandidate>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum FindingSelectionReport {
    Explanation(Box<FindingExplanationReport>),
    Ambiguity(FindingAmbiguityReport),
}

pub fn build_finding_explanation_from_report(
    mcp_root: &Path,
    report_json: &str,
    finding_id: &str,
    source_report: &str,
) -> Result<FindingExplanationReport, String> {
    match build_finding_explanation_selection_from_report(
        mcp_root,
        report_json,
        finding_id,
        source_report,
        None,
    )? {
        FindingSelectionReport::Explanation(report) => Ok(*report),
        FindingSelectionReport::Ambiguity(ambiguity) => Err(ambiguity.message),
    }
}

pub fn build_finding_explanation_selection_from_report(
    mcp_root: &Path,
    report_json: &str,
    finding_id: &str,
    source_report: &str,
    locator: Option<&FindingOccurrenceLocator>,
) -> Result<FindingSelectionReport, String> {
    if finding_id.trim().is_empty() {
        return Err("`finding_id` must not be empty".to_string());
    }

    let report: Value = serde_json::from_str(report_json)
        .map_err(|error| format!("invalid session report: {error}"))?;
    let findings = report
        .get("findings")
        .and_then(Value::as_array)
        .ok_or_else(|| "session report does not contain a findings array".to_string())?;

    let id_matches = findings
        .iter()
        .filter(|finding| finding.get("id").and_then(Value::as_str) == Some(finding_id))
        .collect::<Vec<_>>();

    if id_matches.is_empty() {
        return Err(format!(
            "finding `{finding_id}` was not found in {source_report}"
        ));
    }

    let active_locator = locator.filter(|locator| !locator.is_empty());
    let selected_matches = id_matches
        .iter()
        .copied()
        .filter(|finding| {
            active_locator.is_none_or(|locator| finding_matches_locator(finding, locator))
        })
        .collect::<Vec<_>>();

    if selected_matches.is_empty() {
        return Err(format!(
            "finding `{finding_id}` has no occurrence matching {} in {source_report}",
            locator_description(active_locator.expect("active locator"))
        ));
    }

    if selected_matches.len() > 1 {
        let mut candidates = selected_matches
            .iter()
            .map(|finding| occurrence_candidate(finding))
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            left.evidence_path
                .cmp(&right.evidence_path)
                .then(left.line_start.cmp(&right.line_start))
                .then(left.title.cmp(&right.title))
        });

        let message = format!(
            "finding `{finding_id}` appears more than once in {source_report}; \
             provide `evidence_path` and `line_start` from one candidate"
        );
        return Ok(FindingSelectionReport::Ambiguity(FindingAmbiguityReport {
            status: "ambiguous".to_string(),
            source_report: source_report.to_string(),
            finding_id: finding_id.to_string(),
            message,
            candidates,
        }));
    }

    let finding_value = (*selected_matches[0]).clone();

    // Review JSON flattens Finding and appends in_diff/baseline_status fields.
    // Serde ignores those additive fields while preserving the complete Finding.
    let finding: Finding = serde_json::from_value(finding_value)
        .map_err(|error| format!("invalid finding `{finding_id}`: {error}"))?;

    if finding.provenance.analysis_scope != AnalysisScope::File {
        return Err(format!(
            "finding `{finding_id}` uses `{}` analysis scope; \
             repopilot_explain_finding currently supports file-scoped findings only",
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

    let confinement = crate::path_security::RootConfinement::named(mcp_root, "MCP root")?;
    let source_root = resolve_source_root(&confinement, &report)?;
    let evidence = finding
        .evidence
        .first()
        .ok_or_else(|| format!("finding `{finding_id}` has no evidence path"))?;
    let source_path = resolve_source_path(&confinement, &source_root, &evidence.path)?;

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

    let finding_occurrence_key = occurrence_key(&finding);
    let decision = build_decision_record(&finding);

    Ok(FindingSelectionReport::Explanation(Box::new(
        FindingExplanationReport {
            source_report: source_report.to_string(),
            source_schema_version: report
                .get("schema_version")
                .and_then(Value::as_str)
                .map(str::to_string),
            source_root: source_root.to_string_lossy().to_string(),
            finding,
            occurrence_key: finding_occurrence_key,
            decision,
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
        },
    )))
}

fn finding_matches_locator(finding: &Value, locator: &FindingOccurrenceLocator) -> bool {
    let evidence = finding
        .get("evidence")
        .and_then(Value::as_array)
        .and_then(|evidence| evidence.first());

    if let Some(expected_path) = locator.evidence_path.as_deref() {
        let Some(actual_path) = evidence
            .and_then(|evidence| evidence.get("path"))
            .and_then(Value::as_str)
        else {
            return false;
        };
        if normalize_report_path(actual_path) != normalize_report_path(expected_path) {
            return false;
        }
    }

    if let Some(expected_line) = locator.line_start {
        let Some(actual_line) = evidence
            .and_then(|evidence| evidence.get("line_start"))
            .and_then(Value::as_u64)
            .and_then(|line| usize::try_from(line).ok())
        else {
            return false;
        };
        if actual_line != expected_line {
            return false;
        }
    }

    true
}

fn occurrence_candidate(finding: &Value) -> FindingOccurrenceCandidate {
    let evidence = finding
        .get("evidence")
        .and_then(Value::as_array)
        .and_then(|evidence| evidence.first());

    FindingOccurrenceCandidate {
        evidence_path: evidence
            .and_then(|evidence| evidence.get("path"))
            .and_then(Value::as_str)
            .map(normalize_report_path),
        line_start: evidence
            .and_then(|evidence| evidence.get("line_start"))
            .and_then(Value::as_u64)
            .and_then(|line| usize::try_from(line).ok()),
        line_end: evidence
            .and_then(|evidence| evidence.get("line_end"))
            .and_then(Value::as_u64)
            .and_then(|line| usize::try_from(line).ok()),
        rule_id: finding
            .get("rule_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        title: finding
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
    }
}

fn locator_description(locator: &FindingOccurrenceLocator) -> String {
    let mut parts = Vec::new();
    if let Some(path) = locator.evidence_path.as_deref() {
        parts.push(format!("evidence_path={}", normalize_report_path(path)));
    }
    if let Some(line) = locator.line_start {
        parts.push(format!("line_start={line}"));
    }
    parts.join(", ")
}

fn normalize_report_path(path: &str) -> String {
    path.replace('\\', "/")
}

fn resolve_source_root(
    confinement: &crate::path_security::RootConfinement,
    report: &Value,
) -> Result<PathBuf, String> {
    let value = report
        .get("root_path")
        .and_then(Value::as_str)
        .unwrap_or(".");
    confinement.resolve_allow_missing(Path::new(value), "session report root")
}

fn resolve_source_path(
    confinement: &crate::path_security::RootConfinement,
    source_root: &Path,
    evidence_path: &Path,
) -> Result<PathBuf, String> {
    let candidate = if evidence_path.is_absolute() {
        evidence_path.to_path_buf()
    } else {
        source_root.join(evidence_path)
    };
    confinement.resolve_existing(&candidate, "finding evidence file")
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
