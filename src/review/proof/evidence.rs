use super::{ChangeProof, ChangeProofVerdict, ProofCapabilityStatus, ProofCoverage, ProofScope};
use crate::report::schema::{REPOPILOT_VERSION, SCAN_REPORT_SCHEMA_VERSION};
use crate::review::model::ReviewReport;
use crate::scan::types::ScanMode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

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
    pub selected_checks: Vec<String>,
    pub canonical_projection_hash: String,
    pub unavailable_inputs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceSummary {
    pub class: EvidenceClass,
    pub coverage_status: EvidenceCoverageStatus,
    pub scope: ProofCoverage,
    pub provenance: EvidenceProvenance,
}

impl EvidenceSummary {
    pub fn from_review(report: &ReviewReport, proof: &ChangeProof) -> Self {
        let base_ref = report.summary.base_ref.clone();
        let profile = report.summary.visibility_profile.clone();
        let selected_checks = sorted_unique(report.verification_policy.selected.clone());
        let unavailable_inputs = unavailable_inputs(report.summary.mode);
        let canonical_projection_hash = projection_hash(
            report,
            proof,
            base_ref.clone(),
            profile.clone(),
            selected_checks.clone(),
        );
        Self {
            class: classify(proof),
            coverage_status: coverage_status(proof),
            scope: proof.coverage.clone(),
            provenance: EvidenceProvenance {
                analyzer_version: REPOPILOT_VERSION.to_string(),
                report_schema: SCAN_REPORT_SCHEMA_VERSION.to_string(),
                base_ref,
                profile,
                selected_checks,
                canonical_projection_hash,
                unavailable_inputs,
            },
        }
    }

    pub fn scope_line(&self) -> String {
        format!(
            "{}; {}/{} file(s) analyzed; {} excluded, {} unsupported ({})",
            scope_label(self.scope.scope),
            self.scope.analyzed_files,
            self.scope.requested_files,
            self.scope.excluded_files,
            self.scope.unsupported_files,
            self.coverage_status.label(),
        )
    }

    pub fn provenance_line(&self) -> String {
        format!(
            "RepoPilot {}, schema {}; unavailable: {}",
            self.provenance.analyzer_version,
            self.provenance.report_schema,
            self.provenance.unavailable_inputs.join(", "),
        )
    }
}

pub(crate) fn classify(proof: &ChangeProof) -> EvidenceClass {
    match coverage_status(proof) {
        EvidenceCoverageStatus::Unavailable => EvidenceClass::Unknown,
        EvidenceCoverageStatus::Limited => {
            if proof.coverage.analyzed_files > 0 {
                EvidenceClass::Suspicion
            } else {
                EvidenceClass::Unknown
            }
        }
        EvidenceCoverageStatus::Complete => match proof.verdict {
            ChangeProofVerdict::Broken | ChangeProofVerdict::Verified => {
                EvidenceClass::SupportedProof
            }
            ChangeProofVerdict::Review => EvidenceClass::Suspicion,
            ChangeProofVerdict::NotAssessed => EvidenceClass::Unknown,
        },
    }
}

pub(crate) fn coverage_status(proof: &ChangeProof) -> EvidenceCoverageStatus {
    if proof.coverage.analyzed_files == 0 {
        return EvidenceCoverageStatus::Unavailable;
    }
    let capability_gap = proof.capability_coverage.iter().any(|capability| {
        capability.count > 0
            && matches!(
                capability.status,
                ProofCapabilityStatus::Limited | ProofCapabilityStatus::Unavailable
            )
    });
    let accounted_files = proof
        .coverage
        .analyzed_files
        .saturating_add(proof.coverage.excluded_files)
        .saturating_add(proof.coverage.unsupported_files);
    if proof.coverage.excluded_files > 0
        || proof.coverage.unsupported_files > 0
        || accounted_files != proof.coverage.requested_files
        || !proof.obligations.accounted_for()
        || capability_gap
    {
        EvidenceCoverageStatus::Limited
    } else {
        EvidenceCoverageStatus::Complete
    }
}

pub(crate) fn canonical_json_hash<T: Serialize>(value: &T) -> String {
    let value = serde_json::to_value(value).expect("evidence fingerprint must serialize");
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
        changed_paths,
        proof: serde_json::to_value(proof).expect("change proof must serialize"),
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
    changed_paths: Vec<String>,
    proof: Value,
}

fn unavailable_inputs(mode: ScanMode) -> Vec<String> {
    let mut inputs = vec![
        "current revision".to_string(),
        "head revision".to_string(),
        "scanner configuration".to_string(),
        "toolchain".to_string(),
    ];
    if mode == ScanMode::Changed {
        inputs.push("base revision".to_string());
    }
    inputs.sort();
    inputs
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
        Value::String(value) => output
            .push_str(&serde_json::to_string(value).expect("JSON strings must be serializable")),
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
                    &serde_json::to_string(key).expect("JSON object keys must be serializable"),
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
