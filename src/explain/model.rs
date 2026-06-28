use crate::findings::types::Severity;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ExplainReport {
    pub path: String,
    pub scope: ExplainScope,
    pub source: ExplainSource,
    pub context: ExplainContext,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<ExplainDecision>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ExplainScope {
    pub analysis_scope: String,
    pub decision_source: String,
    pub visibility_profile: String,
    pub repository_context_included: bool,
    pub package_manifest_context_included: bool,
    pub scan_configuration_included: bool,
    pub local_feedback_included: bool,
    pub baseline_included: bool,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ExplainSource {
    pub language_name: Option<String>,
    pub non_empty_lines: usize,
    pub has_inline_tests: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ExplainContext {
    pub language: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_support: Option<String>,
    pub frameworks: Vec<String>,
    pub roles: Vec<String>,
    pub role_evidence: Vec<ExplainRoleEvidence>,
    pub paradigms: Vec<String>,
    pub runtimes: Vec<String>,
    pub is_test: bool,
    pub is_production_code: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub architecture_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_family: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ExplainRoleEvidence {
    pub role: String,
    pub source: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ExplainDecision {
    pub rule_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal: Option<String>,
    pub base_severity: Severity,
    pub action: String,
    pub final_severity: Severity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_signal: Option<ExplainRiskSignal>,
    pub trace: Vec<ExplainDecisionTraceStep>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<ExplainVisibility>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ExplainDecisionTraceStep {
    pub order: usize,
    pub stage: String,
    pub status: String,
    pub label: String,
    pub criteria: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    pub severity_before: Severity,
    pub severity_after: Severity,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub override_index: Option<usize>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ExplainRiskSignal {
    pub id: String,
    pub label: String,
    pub weight: i16,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ExplainVisibility {
    pub profile: String,
    pub intent: String,
    pub visible_by_default: bool,
    pub reason: String,
}
