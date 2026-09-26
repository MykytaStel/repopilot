use super::super::{
    ChangeProof, ChangeProofReason, ChangeProofReasonCode, ChangeProofVerdict, ProofCapability,
    ProofCapabilityStatus, ProofCoverage, ProofObligations, ProofScope,
};
use crate::review::diff::{ChangeStatus, ChangedFile};
use crate::review::model::ReviewReport;
use crate::review::verification::{VerificationPolicy, VerificationPolicyCheck};
use crate::scan::types::{ScanMetadata, ScanMode, ScanSummary};
use crate::verification::VerificationRole;
use serde_json::json;
use std::path::PathBuf;

#[test]
fn evidence_class_requires_complete_supported_scope() {
    assert_eq!(
        super::classify(&proof(ChangeProofVerdict::Verified, 1, 1, 0, 0, vec![])),
        super::EvidenceClass::SupportedProof
    );
    assert_eq!(
        super::classify(&proof(ChangeProofVerdict::Broken, 2, 1, 0, 1, vec![])),
        super::EvidenceClass::Suspicion
    );
    assert_eq!(
        super::classify(&proof(ChangeProofVerdict::NotAssessed, 0, 0, 0, 0, vec![])),
        super::EvidenceClass::Unknown
    );
    assert_eq!(
        super::classify(&proof(
            ChangeProofVerdict::Verified,
            1,
            1,
            0,
            0,
            vec![limited_capability()],
        )),
        super::EvidenceClass::Suspicion
    );
}

#[test]
fn coverage_status_exposes_limits_and_unavailable_scope() {
    assert_eq!(
        super::coverage_status(&proof(ChangeProofVerdict::Review, 1, 1, 1, 0, vec![])),
        super::EvidenceCoverageStatus::Limited
    );
    assert_eq!(
        super::coverage_status(&proof(ChangeProofVerdict::Review, 2, 1, 0, 0, vec![])),
        super::EvidenceCoverageStatus::Limited
    );
    assert_eq!(
        super::coverage_status(&proof(ChangeProofVerdict::NotAssessed, 0, 0, 0, 0, vec![])),
        super::EvidenceCoverageStatus::Unavailable
    );
}

#[test]
fn coverage_limits_name_file_and_verification_gaps() {
    let mut report = base_report();
    report.summary.metrics.large_files_skipped = 1;
    report.verification_policy.selected.clear();
    let mut proof = proof(ChangeProofVerdict::Review, 4, 2, 1, 1, Vec::new());
    proof.reasons.push(ChangeProofReason::new(
        ChangeProofReasonCode::InsufficientPolicy,
        1,
        "No sufficient proof policy was selected for this assessment.",
    ));

    let summary = super::EvidenceSummary::from_review(&report, &proof);

    assert_eq!(
        summary.coverage_status,
        super::EvidenceCoverageStatus::Limited
    );
    assert_eq!(
        summary
            .coverage_limits
            .iter()
            .map(|limit| (limit.code.as_str(), limit.count))
            .collect::<Vec<_>>(),
        [
            ("files-over-size-limit", 1),
            ("unsupported-files", 1),
            ("verification-not-configured", 1),
        ]
    );
    assert!(
        summary
            .scope_line()
            .contains("1 file exceeded the configured size limit")
    );
    assert!(
        summary
            .scope_line()
            .contains("1 file received no supported analysis result")
    );
    assert!(
        summary
            .scope_line()
            .contains("Verification checks are not configured")
    );

    let value = serde_json::to_value(summary).expect("evidence summary JSON");
    assert_eq!(value["coverage_limits"][0]["code"], "files-over-size-limit");
    assert_eq!(value["coverage_limits"][0]["count"], 1);
}

#[test]
fn older_evidence_payloads_decode_without_coverage_limits() {
    let report = base_report();
    let proof = proof(ChangeProofVerdict::Verified, 1, 1, 0, 0, Vec::new());
    let mut value = serde_json::to_value(super::EvidenceSummary::from_review(&report, &proof))
        .expect("evidence summary JSON");
    value
        .as_object_mut()
        .expect("evidence summary object")
        .remove("coverage_limits");

    let decoded: super::EvidenceSummary =
        serde_json::from_value(value).expect("older evidence summary decodes");

    assert!(decoded.coverage_limits.is_empty());
}

#[test]
fn coverage_limits_keep_each_recorded_exclusion_cause() {
    let mut report = base_report();
    report.summary.metrics.large_files_skipped = 1;
    report.summary.metrics.binary_files_skipped = 1;
    report.summary.metrics.files_skipped_by_limit = 1;
    report.summary.metrics.files_skipped_repopilotignore = 1;
    let proof = proof(ChangeProofVerdict::Review, 5, 1, 4, 0, Vec::new());

    let summary = super::EvidenceSummary::from_review(&report, &proof);
    let reasons = summary
        .coverage_limits
        .iter()
        .map(|limit| limit.code.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        reasons,
        [
            "binary-files-skipped",
            "files-over-max-files-limit",
            "files-over-size-limit",
            "files-repopilotignore",
        ]
    );
    assert_eq!(
        summary
            .coverage_limits
            .iter()
            .map(|limit| limit.message.as_str())
            .collect::<Vec<_>>(),
        [
            "1 file was identified as binary.",
            "1 file was omitted by the max-files limit.",
            "1 file exceeded the configured size limit.",
            "1 file was excluded by .repopilotignore.",
        ]
    );
}

#[test]
fn coverage_limits_do_not_guess_when_exclusion_cause_is_missing() {
    let report = base_report();
    let proof = proof(ChangeProofVerdict::Review, 3, 1, 1, 1, Vec::new());

    let summary = super::EvidenceSummary::from_review(&report, &proof);

    assert!(
        summary.coverage_limits.iter().any(|limit| {
            limit.code == "excluded-files-unclassified"
                && limit.message == "1 excluded file has no recorded skip reason."
        }),
        "limits: {:?}",
        summary.coverage_limits
    );
}

#[test]
fn coverage_limits_distinguish_configured_but_unselected_checks() {
    let mut report = base_report();
    report.verification_policy.configured = vec![VerificationPolicyCheck {
        id: "ci-tests".to_string(),
        role: VerificationRole::Test,
        paths: Vec::new(),
    }];
    report.verification_policy.selected.clear();
    let mut proof = proof(ChangeProofVerdict::Review, 1, 1, 0, 0, Vec::new());
    proof.reasons.push(ChangeProofReason::new(
        ChangeProofReasonCode::InsufficientPolicy,
        1,
        "No sufficient proof policy was selected for this assessment.",
    ));

    let summary = super::EvidenceSummary::from_review(&report, &proof);

    assert!(summary.coverage_limits.iter().any(|limit| {
        limit.code == "verification-not-selected"
            && limit.message == "No verification checks are selected."
    }));
}

#[test]
fn canonical_hash_tracks_the_reason_coverage_is_limited() {
    let mut unconfigured = base_report();
    unconfigured.verification_policy.selected.clear();
    let mut configured = base_report();
    configured.verification_policy.selected.clear();
    configured.verification_policy.configured = vec![VerificationPolicyCheck {
        id: "ci-tests".to_string(),
        role: VerificationRole::Test,
        paths: Vec::new(),
    }];
    let mut proof = proof(ChangeProofVerdict::Review, 1, 1, 0, 0, Vec::new());
    proof.reasons.push(ChangeProofReason::new(
        ChangeProofReasonCode::InsufficientPolicy,
        1,
        "No sufficient proof policy was selected for this assessment.",
    ));

    let unconfigured_hash = super::EvidenceSummary::from_review(&unconfigured, &proof)
        .provenance
        .canonical_projection_hash;
    let configured_hash = super::EvidenceSummary::from_review(&configured, &proof)
        .provenance
        .canonical_projection_hash;

    assert_ne!(unconfigured_hash, configured_hash);
}

#[test]
fn deliberate_policy_skips_are_disclosed_without_becoming_coverage_limits() {
    let report = base_report();
    let mut proof = proof(ChangeProofVerdict::Verified, 2, 1, 0, 0, Vec::new());
    proof.coverage.policy_skipped_files = 1;

    let summary = super::EvidenceSummary::from_review(&report, &proof);

    assert_eq!(
        summary.coverage_status,
        super::EvidenceCoverageStatus::Complete
    );
    assert!(summary.coverage_limits.is_empty());
    assert!(
        summary
            .scope_line()
            .contains("1 test/fixture/generated skipped by policy")
    );
}

#[test]
fn canonical_hash_ignores_object_and_collection_order() {
    let first = json!({"paths": ["b", "a"], "meta": {"z": 1, "a": 2}});
    let second = json!({"meta": {"a": 2, "z": 1}, "paths": ["a", "b"]});
    assert_eq!(
        super::canonical_json_hash(&first),
        super::canonical_json_hash(&second)
    );
    assert_ne!(
        super::canonical_json_hash(&first),
        super::canonical_json_hash(&json!({"paths": ["a", "c"]}))
    );
}

#[test]
fn summary_records_provenance_and_normalizes_collection_order() {
    let mut report = base_report();
    report.summary.metrics.files_discovered = 2;
    report.summary.metrics.files_analyzed = 2;
    let proof = proof(ChangeProofVerdict::Review, 2, 2, 0, 0, Vec::new());

    let first = super::EvidenceSummary::from_review(&report, &proof);
    assert_eq!(first.provenance.base_ref.as_deref(), Some("origin/main"));
    assert_eq!(first.provenance.profile.as_deref(), Some("default"));
    assert_eq!(first.provenance.selected_checks, ["a", "z"]);
    assert!(
        first
            .provenance
            .canonical_projection_hash
            .starts_with("sha256:")
    );
    assert!(
        first
            .provenance
            .unavailable_inputs
            .contains(&"current revision".to_string())
    );

    report.changed_files.reverse();
    report.verification_policy.selected.reverse();
    let second = super::EvidenceSummary::from_review(&report, &proof);
    assert_eq!(
        first.provenance.canonical_projection_hash,
        second.provenance.canonical_projection_hash
    );
}

#[test]
fn recorded_revisions_are_not_reported_unavailable() {
    let mut report = base_report();
    report.summary.mode = crate::scan::types::ScanMode::Changed;
    report.analysis_revision = Some("workspace-rev".to_string());
    report.revisions = crate::review::model::ReviewRevisions {
        base_commit: Some("1f17e2c7aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
        head_commit: Some("6a49e4bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string()),
        head_is_working_tree: false,
    };
    let proof = proof(ChangeProofVerdict::Review, 1, 1, 0, 0, Vec::new());

    let evidence = super::EvidenceSummary::from_review(&report, &proof);
    assert_eq!(
        evidence.provenance.unavailable_inputs,
        ["scanner configuration", "toolchain"]
    );
    assert_eq!(
        evidence.provenance_line(),
        format!(
            "RepoPilot {}, schema {}; base 1f17e2c7, head 6a49e4bb; unavailable: scanner configuration, toolchain",
            crate::report::schema::REPOPILOT_VERSION,
            crate::report::schema::SCAN_REPORT_SCHEMA_VERSION
        )
    );
}

#[test]
fn unresolved_refs_stay_unavailable() {
    let mut report = base_report();
    report.summary.mode = crate::scan::types::ScanMode::Changed;
    let proof = proof(ChangeProofVerdict::Review, 1, 1, 0, 0, Vec::new());

    let evidence = super::EvidenceSummary::from_review(&report, &proof);
    for input in ["base revision", "current revision", "head revision"] {
        assert!(
            evidence
                .provenance
                .unavailable_inputs
                .contains(&input.to_string())
        );
    }
}

fn base_report() -> ReviewReport {
    ReviewReport {
        analysis_revision: None,
        revisions: Default::default(),
        summary: ScanSummary {
            metadata: ScanMetadata {
                mode: ScanMode::Changed,
                base_ref: Some("origin/main".to_string()),
                visibility_profile: Some("default".to_string()),
                ..Default::default()
            },
            ..Default::default()
        },
        repo_root: PathBuf::from("/repo"),
        baseline_path: None,
        changed_files: vec![changed_file("b.rs"), changed_file("a.rs")],
        blast_radius: Vec::new(),
        impact_paths: Default::default(),
        ownership: Default::default(),
        ownership_diagnostics: Vec::new(),
        boundary_signals: Vec::new(),
        boundary_missing_test: false,
        tiered_signals: Default::default(),
        timings: Default::default(),
        verification_policy: VerificationPolicy {
            configured: Vec::new(),
            selected: vec!["z".to_string(), "a".to_string()],
        },
        verification: Vec::new(),
        intent: Default::default(),
        findings: Vec::new(),
    }
}

fn changed_file(path: &str) -> ChangedFile {
    ChangedFile {
        path: PathBuf::from(path),
        status: ChangeStatus::Modified,
        ranges: Vec::new(),
        hunks: Vec::new(),
    }
}

fn proof(
    verdict: ChangeProofVerdict,
    requested_files: usize,
    analyzed_files: usize,
    excluded_files: usize,
    unsupported_files: usize,
    capability_coverage: Vec<ProofCapability>,
) -> ChangeProof {
    ChangeProof {
        verdict,
        reasons: Vec::new(),
        coverage: ProofCoverage {
            scope: ProofScope::Changed,
            requested_files,
            analyzed_files,
            excluded_files,
            unsupported_files,
            policy_skipped_files: 0,
        },
        obligations: ProofObligations {
            applicable: 0,
            satisfied: 0,
            failed: 0,
            unavailable: 0,
            unselected: 0,
            stale: 0,
        },
        contract_deltas: Vec::new(),
        capability_coverage,
        intent_drift: Default::default(),
    }
}

fn limited_capability() -> ProofCapability {
    ProofCapability {
        id: "contract-deltas".to_string(),
        status: ProofCapabilityStatus::Limited,
        count: 1,
        message: String::new(),
    }
}

#[test]
fn windows_separators_do_not_change_evidence_identity() {
    let proof = proof(ChangeProofVerdict::Review, 1, 1, 0, 0, Vec::new());
    let mut unix = base_report();
    unix.changed_files = vec![changed_file("src/auth/session.rs")];
    let mut windows = base_report();
    windows.changed_files = vec![changed_file("src\\auth\\session.rs")];

    assert_eq!(
        windows.changed_files[0].path_string(),
        "src/auth/session.rs"
    );
    assert_eq!(
        super::EvidenceSummary::from_review(&unix, &proof)
            .provenance
            .canonical_projection_hash,
        super::EvidenceSummary::from_review(&windows, &proof)
            .provenance
            .canonical_projection_hash
    );
}
