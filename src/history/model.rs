use crate::baseline::key::{normalized_relative_path, stable_finding_key};
use crate::findings::occurrence::occurrence_key;
use crate::findings::types::{Finding, Severity};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const HISTORY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AnalysisScope {
    Full,
    Workspace,
    Changed,
    ReviewChanged,
    ReviewFull,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComparisonIdentity {
    pub workspace: String,
    pub analysis_target: String,
    pub scope: AnalysisScope,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_revision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head_revision: Option<String>,
    pub profile: String,
    pub config_fingerprint: String,
    pub selection_fingerprint: String,
    pub overlay_fingerprint: String,
    pub analysis_schema: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FindingReceipt {
    pub baseline_id: String,
    pub occurrence_key: String,
    pub rule_id: String,
    pub severity: Severity,
    pub path: String,
}

impl FindingReceipt {
    pub fn from_finding(finding: &Finding, root: &Path) -> Self {
        let path = finding
            .evidence
            .first()
            .map(|evidence| normalized_relative_path(&evidence.path, root))
            .unwrap_or_else(|| ".".to_string());
        Self {
            baseline_id: stable_finding_key(finding, root),
            occurrence_key: occurrence_key(finding),
            rule_id: finding.rule_id.clone(),
            severity: finding.severity,
            path,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunReceipt {
    pub schema_version: u32,
    pub comparison: ComparisonIdentity,
    pub recorded_at: String,
    pub revision: String,
    pub findings: Vec<FindingReceipt>,
}

impl RunReceipt {
    pub fn new(
        comparison: ComparisonIdentity,
        recorded_at: String,
        revision: String,
        findings: impl IntoIterator<Item = FindingReceipt>,
    ) -> Self {
        let mut findings = findings.into_iter().collect::<Vec<_>>();
        findings.sort_by(|left, right| {
            (
                &left.occurrence_key,
                &left.rule_id,
                &left.path,
                &left.baseline_id,
            )
                .cmp(&(
                    &right.occurrence_key,
                    &right.rule_id,
                    &right.path,
                    &right.baseline_id,
                ))
        });
        Self {
            schema_version: HISTORY_SCHEMA_VERSION,
            comparison,
            recorded_at,
            revision,
            findings,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ComparisonUnavailable {
    SchemaMismatch,
    WorkspaceMismatch,
    TargetMismatch,
    ScopeMismatch,
    RevisionRangeMismatch,
    ProfileMismatch,
    ConfigMismatch,
    SelectionMismatch,
    OverlayMismatch,
    AnalysisSchemaMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", content = "value", rename_all = "kebab-case")]
pub enum ComparisonResult {
    Compatible(RiskDelta),
    Unavailable(ComparisonUnavailable),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SeverityShift {
    pub occurrence_key: String,
    pub rule_id: String,
    pub path: String,
    pub old_severity: Severity,
    pub new_severity: Severity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ComparisonProvenance {
    pub prior_revision: String,
    pub current_revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RiskDelta {
    pub comparison: ComparisonProvenance,
    pub new_findings: Vec<FindingReceipt>,
    pub persisting_findings: Vec<FindingReceipt>,
    pub resolved_findings: Vec<FindingReceipt>,
    pub severity_shifts: Vec<SeverityShift>,
}

impl RiskDelta {
    pub fn has_changes(&self) -> bool {
        !self.new_findings.is_empty()
            || !self.resolved_findings.is_empty()
            || !self.severity_shifts.is_empty()
    }
}
