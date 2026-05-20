use repopilot::findings::contract::{
    FindingContractViolationKind, validate_finding_contract, validate_findings_contract,
};
use repopilot::findings::types::{Confidence, Evidence, Finding, FindingCategory, Severity};
use repopilot::risk::{RiskInputs, assess_finding};
use repopilot::scan::config::ScanConfig;
use repopilot::scan::scanner::scan_path_with_config;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

fn valid_finding() -> Finding {
    let mut finding = Finding {
        id: "security.secret-candidate:src/config.rs:1".to_string(),
        rule_id: "security.secret-candidate".to_string(),
        title: "Possible secret detected".to_string(),
        description: "A hardcoded secret was found.".to_string(),
        recommendation: Finding::recommendation_for_rule_id("security.secret-candidate"),
        category: FindingCategory::Security,
        severity: Severity::High,
        confidence: Confidence::High,
        evidence: vec![Evidence {
            path: PathBuf::from("src/config.rs"),
            line_start: 1,
            line_end: Some(1),
            snippet: "API_KEY = \"sk_live\"".to_string(),
        }],
        workspace_package: None,
        docs_url: Some("https://example.com/secrets".to_string()),
        provenance: Default::default(),
        risk: Default::default(),
    };
    finding.risk = assess_finding(&finding, None, RiskInputs::default());
    finding
}

#[test]
fn valid_finding_passes_contract() {
    let finding = valid_finding();
    let report = validate_findings_contract(&[finding]);

    assert_eq!(report.findings_checked, 1);
    assert_eq!(report.valid_findings, 1);
    assert_eq!(report.invalid_findings, 0);
    assert!(report.violations.is_empty());
}

#[test]
fn invalid_finding_reports_all_required_contract_gaps() {
    let mut finding = Finding {
        severity: Severity::High,
        evidence: vec![Evidence {
            path: PathBuf::new(),
            line_start: 0,
            line_end: Some(0),
            snippet: String::new(),
        }],
        ..Finding::default()
    };
    finding.risk.formula_version.clear();

    let violations = validate_finding_contract(&finding)
        .into_iter()
        .map(|violation| violation.violation)
        .collect::<Vec<_>>();

    for expected in [
        FindingContractViolationKind::EmptyId,
        FindingContractViolationKind::EmptyRuleId,
        FindingContractViolationKind::EmptyTitle,
        FindingContractViolationKind::EmptyDescription,
        FindingContractViolationKind::EmptyRecommendation,
        FindingContractViolationKind::InvalidEvidencePath,
        FindingContractViolationKind::InvalidEvidenceLineRange,
        FindingContractViolationKind::MissingRiskFormulaVersion,
        FindingContractViolationKind::MissingRiskSignals,
        FindingContractViolationKind::MissingDocsForHighSeverity,
    ] {
        assert!(
            violations.contains(&expected),
            "missing expected violation {expected:?} in {violations:?}"
        );
    }
}

#[test]
fn generated_findings_from_scan_pass_contract() {
    let temp = tempdir().expect("temp dir");
    fs::create_dir_all(temp.path().join("src")).expect("src dir");
    fs::write(
        temp.path().join("src/config.rs"),
        "pub const API_KEY: &str = \"sk_live_repopilot_contract_1234567890abcdef\";\n",
    )
    .expect("write source");

    let summary = scan_path_with_config(
        temp.path(),
        &ScanConfig {
            detect_missing_tests: false,
            ..ScanConfig::default()
        },
    )
    .expect("scan");

    assert!(
        summary
            .findings
            .iter()
            .any(|finding| finding.rule_id == "security.secret-candidate"),
        "fixture should produce a generated security finding"
    );
    let report = validate_findings_contract(&summary.findings);
    assert!(
        report.violations.is_empty(),
        "generated findings should satisfy contract: {:?}",
        report.violations
    );
}
