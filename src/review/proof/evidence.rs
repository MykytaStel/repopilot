use super::{ChangeProof, ProofCoverage, ProofScope};
use crate::report::schema::{REPOPILOT_VERSION, SCAN_REPORT_SCHEMA_VERSION};
use crate::review::model::ReviewReport;
use crate::scan::types::ScanMode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

#[path = "evidence/coverage.rs"]
mod coverage;
#[path = "evidence_inputs.rs"]
mod inputs;
pub use coverage::EvidenceCoverageLimit;
use coverage::coverage_limits;
pub(crate) use coverage::{classify, coverage_status};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
/// A claim-strength label; supported static proof does not imply all runtime
/// properties of the changed system.
pub enum EvidenceClass {
    Observation,
    SupportedProof,
    Suspicion,
    Unknown,
}

impl EvidenceClass {
    pub fn label(self) -> &'static str {
        match self {
            Self::Observation => "OBSERVATION",
            Self::SupportedProof => "SUPPORTED PROOF",
            Self::Suspicion => "SUSPICION",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceCoverageStatus {
    Complete,
    Limited,
    Unavailable,
}

impl EvidenceCoverageStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Limited => "limited",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceProvenance {
    pub analyzer_version: String,
    pub report_schema: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_commit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head_commit: Option<String>,
    pub selected_checks: Vec<String>,
    pub canonical_projection_hash: String,
    pub unavailable_inputs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceSummary {
    pub class: EvidenceClass,
    pub coverage_status: EvidenceCoverageStatus,
    pub scope: ProofCoverage,
    #[serde(default)]
    pub coverage_limits: Vec<EvidenceCoverageLimit>,
    pub provenance: EvidenceProvenance,
}

impl EvidenceSummary {
    pub fn from_review(report: &ReviewReport, proof: &ChangeProof) -> Self {
        let base_ref = report.summary.base_ref.clone();
        let profile = report.summary.visibility_profile.clone();
        let selected_checks = sorted_unique(report.verification_policy.selected.clone());
        let unavailable_inputs = inputs::unavailable_inputs(report);
        let coverage_limits = coverage_limits(report, proof);
        let canonical_projection_hash = projection_hash(
            report,
            proof,
            base_ref.clone(),
            profile.clone(),
            selected_checks.clone(),
            coverage_limits.clone(),
        );
        Self {
            class: classify(proof),
            coverage_status: coverage_status(proof),
            scope: proof.coverage.clone(),
            coverage_limits,
            provenance: EvidenceProvenance {
                analyzer_version: REPOPILOT_VERSION.to_string(),
                report_schema: SCAN_REPORT_SCHEMA_VERSION.to_string(),
                base_ref,
                profile,
                base_commit: report.revisions.base_commit.clone(),
                head_commit: report.revisions.head_commit.clone(),
                selected_checks,
                canonical_projection_hash,
                unavailable_inputs,
            },
        }
    }

    pub fn scope_line(&self) -> String {
        let policy_skipped = match self.scope.policy_skipped_files {
            0 => String::new(),
            count => format!(", {count} test/fixture/generated skipped by policy"),
        };
        let status = if self.coverage_limits.is_empty() {
            self.coverage_status.label().to_string()
        } else {
            format!(
                "{}: {}",
                self.coverage_status.label(),
                self.coverage_limits
                    .iter()
                    .map(|limit| limit.message.as_str())
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        };
        format!(
            "{}; {}/{} file(s) analyzed; {} excluded, {} unsupported{} ({status})",
            scope_label(self.scope.scope),
            self.scope.analyzed_files,
            self.scope.requested_files,
            self.scope.excluded_files,
            self.scope.unsupported_files,
            policy_skipped,
        )
    }

    pub fn provenance_line(&self) -> String {
        let mut line = format!(
            "RepoPilot {}, schema {}",
            self.provenance.analyzer_version, self.provenance.report_schema,
        );
        if let Some(revisions) = inputs::revision_summary(
            self.provenance.base_commit.as_deref(),
            self.provenance.head_commit.as_deref(),
        ) {
            line.push_str(&format!("; {revisions}"));
        }
        if !self.provenance.unavailable_inputs.is_empty() {
            line.push_str(&format!(
                "; unavailable: {}",
                self.provenance.unavailable_inputs.join(", ")
            ));
        }
        line
    }
}

pub(crate) fn canonical_json_hash<T: Serialize>(value: &T) -> String {
    let value = serde_json::to_value(value).unwrap_or(Value::Null);
    let mut canonical = String::new();
    write_canonical_json(&value, &mut canonical);
    let digest = Sha256::digest(canonical.as_bytes());
    let encoded = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("sha256:{encoded}")
}

fn projection_hash(
    report: &ReviewReport,
    proof: &ChangeProof,
    base_ref: Option<String>,
    profile: Option<String>,
    selected_checks: Vec<String>,
    coverage_limits: Vec<EvidenceCoverageLimit>,
) -> String {
    let changed_paths = sorted_unique(
        report
            .changed_files
            .iter()
            .map(|file| file.path_string())
            .collect::<Vec<_>>(),
    );
    canonical_json_hash(&EvidenceFingerprint {
        analyzer_version: REPOPILOT_VERSION,
        report_schema: SCAN_REPORT_SCHEMA_VERSION,
        mode: report.summary.mode,
        base_ref,
        profile,
        coverage: proof.coverage.clone(),
        selected_checks,
        coverage_limits,
        changed_paths,
        proof: serde_json::to_value(proof).unwrap_or(Value::Null),
    })
}

#[derive(Debug, Serialize)]
struct EvidenceFingerprint {
    analyzer_version: &'static str,
    report_schema: &'static str,
    mode: ScanMode,
    base_ref: Option<String>,
    profile: Option<String>,
    coverage: ProofCoverage,
    selected_checks: Vec<String>,
    coverage_limits: Vec<EvidenceCoverageLimit>,
    changed_paths: Vec<String>,
    proof: Value,
}

fn scope_label(scope: ProofScope) -> &'static str {
    match scope {
        ProofScope::Changed => "changed",
        ProofScope::Full => "full",
    }
}

fn sorted_unique(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values.dedup();
    values
}

fn write_canonical_json(value: &Value, output: &mut String) {
    match value {
        Value::Null => output.push_str("null"),
        Value::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
        Value::Number(value) => output.push_str(&value.to_string()),
        Value::String(value) => output.push_str(
            &serde_json::to_string(value).unwrap_or_else(|_| "\"<invalid>\"".to_string()),
        ),
        Value::Array(values) => {
            let mut items = values
                .iter()
                .map(|value| {
                    let mut rendered = String::new();
                    write_canonical_json(value, &mut rendered);
                    rendered
                })
                .collect::<Vec<_>>();
            items.sort();
            output.push('[');
            output.push_str(&items.join(","));
            output.push(']');
        }
        Value::Object(values) => {
            let mut entries = values.iter().collect::<Vec<_>>();
            entries.sort_by_key(|(left, _)| *left);
            output.push('{');
            for (index, (key, value)) in entries.into_iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                output.push_str(
                    &serde_json::to_string(key).unwrap_or_else(|_| "\"<invalid>\"".to_string()),
                );
                output.push(':');
                write_canonical_json(value, output);
            }
            output.push('}');
        }
    }
}

#[cfg(test)]
#[path = "evidence_tests.rs"]
mod tests;
