use repopilot::findings::quality::summarize_signal_quality;
use repopilot::findings::types::{Confidence, Evidence, Finding, FindingCategory, Severity};
use repopilot::risk::{RiskInputs, assess_finding};
use std::path::PathBuf;

#[test]
fn signal_quality_counts_confidence_coverage_and_contract_warnings() {
    let mut finding = Finding {
        id: "finding-1".to_string(),
        rule_id: "code-quality.long-function".to_string(),
        title: "Long function".to_string(),
        description: "Function is long.".to_string(),
        recommendation: Finding::recommendation_for_rule_id("code-quality.long-function"),
        category: FindingCategory::CodeQuality,
        severity: Severity::Medium,
        confidence: Confidence::High,
        evidence: vec![Evidence {
            path: PathBuf::from("src/lib.rs"),
            line_start: 1,
            line_end: None,
            snippet: String::new(),
        }],
        workspace_package: None,
        docs_url: None,
        provenance: Default::default(),
        risk: Default::default(),
    };
    finding.populate_rule_metadata();
    finding.risk = assess_finding(&finding, None, RiskInputs::default());

    let quality = summarize_signal_quality(&[finding]);

    assert_eq!(quality.findings_total, 1);
    assert_eq!(quality.by_confidence.high, 1);
    assert_eq!(quality.evidence_coverage_percent, 100);
    assert_eq!(quality.recommendation_coverage_percent, 100);
    assert_eq!(quality.contract_violations, 0);
}
