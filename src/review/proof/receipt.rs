use super::evidence::canonical_json_hash;
use super::{ChangeProof, EvidenceSummary, next_action_for};
use crate::report::schema::{REPOPILOT_VERSION, SCAN_REPORT_SCHEMA_VERSION};
use crate::review::model::ReviewReport;
use crate::scan::session::WorkspaceRevision;
use serde::{Deserialize, Serialize};
use serde_json::json;

pub const PROOF_RECEIPT_SCHEMA_VERSION: &str = "0.1";
const MAX_SERIALIZED_RECEIPT_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReceiptReplayState {
    Matched,
    Stale,
    Unsupported,
    Invalid,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptReplayContext {
    pub workspace_revision: Option<String>,
    pub configuration_hash: Option<String>,
    pub analyzer_version: Option<String>,
    pub report_schema: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReceiptReplayDiagnostic {
    pub state: ReceiptReplayState,
    pub code: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofReceipt {
    pub schema_version: String,
    pub analyzer_version: String,
    pub report_schema: String,
    pub workspace_revision: String,
    pub configuration_hash: String,
    pub replay_state: ReceiptReplayState,
    pub projection_hash: String,
    pub proof: ChangeProof,
    pub evidence: EvidenceSummary,
    pub reason_codes: Vec<String>,
    pub next_action: String,
    pub unavailable_inputs: Vec<String>,
}

impl ProofReceipt {
    pub fn replay_context(&self) -> ReceiptReplayContext {
        ReceiptReplayContext {
            workspace_revision: Some(self.workspace_revision.clone()),
            configuration_hash: Some(self.configuration_hash.clone()),
            analyzer_version: Some(self.analyzer_version.clone()),
            report_schema: Some(self.report_schema.clone()),
        }
    }
}

pub fn build_proof_receipt(report: &ReviewReport, proof: &ChangeProof) -> ProofReceipt {
    let proof = canonicalize_proof(proof.clone());
    let evidence = EvidenceSummary::from_review(report, &proof);
    let workspace_revision = report.analysis_revision.clone().unwrap_or_else(|| {
        WorkspaceRevision::capture(&report.repo_root)
            .id()
            .to_string()
    });
    let configuration_hash = configuration_hash(report);
    let reason_codes = sorted_unique(
        proof
            .reasons
            .iter()
            .map(|reason| reason_code(reason.code))
            .collect(),
    );
    let unavailable_inputs = sorted_unique(evidence.provenance.unavailable_inputs.clone());
    let next_action = next_action_for(&proof).to_string();
    let mut receipt = ProofReceipt {
        schema_version: PROOF_RECEIPT_SCHEMA_VERSION.to_string(),
        analyzer_version: REPOPILOT_VERSION.to_string(),
        report_schema: SCAN_REPORT_SCHEMA_VERSION.to_string(),
        workspace_revision,
        configuration_hash,
        replay_state: ReceiptReplayState::Matched,
        projection_hash: String::new(),
        proof,
        evidence,
        reason_codes,
        next_action,
        unavailable_inputs,
    };
    receipt.projection_hash = receipt_projection_hash(&receipt);
    receipt
}

fn canonicalize_proof(mut proof: ChangeProof) -> ChangeProof {
    sort_serialized(&mut proof.reasons);
    sort_serialized(&mut proof.contract_deltas);
    sort_serialized(&mut proof.capability_coverage);
    canonicalize_intent(&mut proof.intent_drift);
    proof
}

fn canonicalize_intent(intent: &mut crate::review::intent::IntentDrift) {
    sort_strings(&mut intent.declared_paths);
    intent.declared_contract_families.sort();
    sort_strings(&mut intent.declared_critical_paths);
    sort_strings(&mut intent.expected_verification);
    sort_strings(&mut intent.actual_paths);
    intent.actual_contract_families.sort();
    for matched in &mut intent.critical_path_matches {
        sort_strings(&mut matched.paths);
    }
    sort_serialized(&mut intent.critical_path_matches);
    sort_strings(&mut intent.unexpected_paths);
    intent.unexpected_contract_families.sort();
    sort_strings(&mut intent.unexpected_critical_paths);
    sort_strings(&mut intent.missing_verification);
}

fn sort_strings(values: &mut [String]) {
    values.sort();
}

fn sort_serialized<T: Serialize>(values: &mut [T]) {
    values.sort_by_key(|value| serde_json::to_string(value).unwrap_or_default());
}

pub fn replay_receipt(
    receipt: &ProofReceipt,
    context: &ReceiptReplayContext,
) -> ReceiptReplayState {
    replay_receipt_with_reason(receipt, context).state
}

pub fn replay_receipt_with_reason(
    receipt: &ProofReceipt,
    context: &ReceiptReplayContext,
) -> ReceiptReplayDiagnostic {
    if receipt.schema_version != PROOF_RECEIPT_SCHEMA_VERSION {
        return diagnostic(
            ReceiptReplayState::Unsupported,
            "receipt-schema-unsupported",
            "receipt schema is not supported",
        );
    }
    let Some(analyzer_version) = context.analyzer_version.as_deref() else {
        return diagnostic(
            ReceiptReplayState::Unavailable,
            "replay-context-unavailable",
            "replay context is missing analyzer version",
        );
    };
    let Some(report_schema) = context.report_schema.as_deref() else {
        return diagnostic(
            ReceiptReplayState::Unavailable,
            "replay-context-unavailable",
            "replay context is missing report schema",
        );
    };
    let Some(workspace_revision) = context.workspace_revision.as_deref() else {
        return diagnostic(
            ReceiptReplayState::Unavailable,
            "replay-context-unavailable",
            "replay context is missing workspace revision",
        );
    };
    let Some(configuration_hash) = context.configuration_hash.as_deref() else {
        return diagnostic(
            ReceiptReplayState::Unavailable,
            "replay-context-unavailable",
            "replay context is missing configuration hash",
        );
    };
    if analyzer_version != REPOPILOT_VERSION || receipt.analyzer_version != REPOPILOT_VERSION {
        return diagnostic(
            ReceiptReplayState::Unsupported,
            "analyzer-unsupported",
            "analyzer version is not supported",
        );
    }
    if report_schema != SCAN_REPORT_SCHEMA_VERSION
        || receipt.report_schema != SCAN_REPORT_SCHEMA_VERSION
    {
        return diagnostic(
            ReceiptReplayState::Unsupported,
            "report-schema-unsupported",
            "report schema is not supported",
        );
    }
    if workspace_revision != receipt.workspace_revision
        || configuration_hash != receipt.configuration_hash
    {
        return diagnostic(
            ReceiptReplayState::Stale,
            "replay-context-stale",
            "workspace revision or configuration hash differs",
        );
    }
    if receipt_projection_hash(receipt) != receipt.projection_hash {
        return diagnostic(
            ReceiptReplayState::Invalid,
            "receipt-hash-invalid",
            "receipt projection hash does not match its payload",
        );
    }
    diagnostic(
        ReceiptReplayState::Matched,
        "receipt-matched",
        "receipt matches the replay context",
    )
}

/// Validate and replay a serialized receipt without touching the filesystem,
/// network, or command execution. Schema dispatch happens before typed decode
/// so a future payload is reported as unsupported even when its shape differs.
pub fn replay_serialized_receipt(
    input: &[u8],
    context: &ReceiptReplayContext,
) -> ReceiptReplayDiagnostic {
    if input.len() > MAX_SERIALIZED_RECEIPT_BYTES {
        return diagnostic(
            ReceiptReplayState::Invalid,
            "receipt-input-too-large",
            "serialized receipt exceeds the supported size limit",
        );
    }

    let value: serde_json::Value = match serde_json::from_slice(input) {
        Ok(value) => value,
        Err(_) => {
            return diagnostic(
                ReceiptReplayState::Invalid,
                "receipt-json-invalid",
                "serialized receipt is not valid JSON",
            );
        }
    };
    let Some(schema_version) = value.get("schema_version").and_then(|value| value.as_str()) else {
        return diagnostic(
            ReceiptReplayState::Invalid,
            "receipt-schema-invalid",
            "serialized receipt does not contain a string schema version",
        );
    };
    if schema_version != PROOF_RECEIPT_SCHEMA_VERSION {
        return diagnostic(
            ReceiptReplayState::Unsupported,
            "receipt-schema-unsupported",
            "receipt schema is not supported",
        );
    }
    let receipt: ProofReceipt = match serde_json::from_value(value) {
        Ok(receipt) => receipt,
        Err(_) => {
            return diagnostic(
                ReceiptReplayState::Invalid,
                "receipt-schema-invalid",
                "serialized receipt does not match the supported schema",
            );
        }
    };
    replay_receipt_with_reason(&receipt, context)
}

fn diagnostic(state: ReceiptReplayState, code: &str, reason: &str) -> ReceiptReplayDiagnostic {
    ReceiptReplayDiagnostic {
        state,
        code: code.to_string(),
        reason: reason.chars().take(256).collect(),
    }
}

fn configuration_hash(report: &ReviewReport) -> String {
    let mut configured_checks = report
        .verification_policy
        .configured
        .iter()
        .map(|check| {
            let mut paths = check.paths.clone();
            paths.sort();
            paths.dedup();
            json!({
                "id": check.id,
                "role": check.role,
                "paths": paths,
            })
        })
        .collect::<Vec<_>>();
    configured_checks.sort_by_key(|check| check.to_string());
    let mut critical_paths = report
        .intent
        .critical_paths
        .iter()
        .map(|rule| {
            let mut paths = rule.paths.clone();
            paths.sort();
            paths.dedup();
            json!({ "name": rule.name, "paths": paths })
        })
        .collect::<Vec<_>>();
    critical_paths.sort_by_key(|rule| rule.to_string());
    let intent_contract = report.intent.contract.as_ref().map(|contract| {
        let mut paths = contract.paths.clone();
        paths.sort();
        let mut contract_families = contract.contract_families.clone();
        contract_families.sort();
        let mut declared_critical_paths = contract.critical_paths.clone();
        declared_critical_paths.sort();
        let mut verification = contract.verification.clone();
        verification.sort();
        json!({
            "version": contract.version,
            "summary": contract.summary,
            "paths": paths,
            "contract_families": contract_families,
            "critical_paths": declared_critical_paths,
            "verification": verification,
        })
    });
    let mut selected_checks = report.verification_policy.selected.clone();
    selected_checks.sort();
    selected_checks.dedup();
    canonical_json_hash(&json!({
        "mode": report.summary.mode,
        "visibility_profile": report.summary.visibility_profile,
        "base_ref": report.summary.base_ref,
        "configured_checks": configured_checks,
        "selected_checks": selected_checks,
        "intent_contract": intent_contract,
        "critical_paths": critical_paths,
        "report_schema": SCAN_REPORT_SCHEMA_VERSION,
    }))
}

fn receipt_projection_hash(receipt: &ProofReceipt) -> String {
    canonical_json_hash(&ReceiptFingerprint {
        schema_version: &receipt.schema_version,
        analyzer_version: &receipt.analyzer_version,
        report_schema: &receipt.report_schema,
        workspace_revision: &receipt.workspace_revision,
        configuration_hash: &receipt.configuration_hash,
        proof: &receipt.proof,
        evidence: &receipt.evidence,
        reason_codes: &receipt.reason_codes,
        next_action: &receipt.next_action,
        unavailable_inputs: &receipt.unavailable_inputs,
    })
}

#[derive(Serialize)]
struct ReceiptFingerprint<'a> {
    schema_version: &'a str,
    analyzer_version: &'a str,
    report_schema: &'a str,
    workspace_revision: &'a str,
    configuration_hash: &'a str,
    proof: &'a ChangeProof,
    evidence: &'a EvidenceSummary,
    reason_codes: &'a [String],
    next_action: &'a str,
    unavailable_inputs: &'a [String],
}

fn reason_code(code: super::ChangeProofReasonCode) -> String {
    serde_json::to_value(code)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| "unknown".to_string())
}

fn sorted_unique(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values.dedup();
    values
}
