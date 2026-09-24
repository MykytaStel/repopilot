use super::super::{
    ChangeProof, ChangeProofVerdict, ProofCapability, ProofCapabilityStatus, ProofCoverage,
    ProofObligations, ProofScope,
};
use crate::review::diff::{ChangeStatus, ChangedFile};
use crate::review::model::ReviewReport;
use crate::review::verification::VerificationPolicy;
use crate::scan::types::{ScanMetadata, ScanMode, ScanSummary};
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
