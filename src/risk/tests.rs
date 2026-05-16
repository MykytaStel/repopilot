use super::*;
use crate::findings::types::{Confidence, Evidence, Finding, Severity};
use crate::scan::facts::FileFacts;
use std::path::PathBuf;

#[test]
fn priority_thresholds_match_v1_plan() {
    assert_eq!(priority_for_score(90), RiskPriority::P0);
    assert_eq!(priority_for_score(70), RiskPriority::P1);
    assert_eq!(priority_for_score(40), RiskPriority::P2);
    assert_eq!(priority_for_score(39), RiskPriority::P3);
}

#[test]
fn high_confidence_critical_score_clamps_to_100() {
    let finding = Finding {
        severity: Severity::Critical,
        confidence: Confidence::High,
        ..Finding::default()
    };

    let assessment = assess_finding(&finding, None, RiskInputs::default());

    assert_eq!(assessment.score, 100);
    assert_eq!(assessment.priority, RiskPriority::P0);
}

#[test]
fn production_domain_file_scores_above_equivalent_test_file() {
    let finding = Finding {
        severity: Severity::Medium,
        confidence: Confidence::Medium,
        evidence: vec![Evidence {
            path: PathBuf::from("src/domain/user.rs"),
            line_start: 1,
            line_end: None,
            snippet: String::new(),
        }],
        ..Finding::default()
    };
    let production = file("src/domain/user.rs", Some("Rust"), false);
    let test = file("tests/user.rs", Some("Rust"), false);

    let prod_risk = assess_finding(&finding, Some(&production), RiskInputs::default());
    let test_risk = assess_finding(&finding, Some(&test), RiskInputs::default());

    assert!(prod_risk.score > test_risk.score);
    assert!(prod_risk.signals.iter().any(|s| s.id == "role.domain"));
    assert!(test_risk.signals.iter().any(|s| s.id == "role.test"));
}

fn file(path: &str, language: Option<&str>, has_inline_tests: bool) -> FileFacts {
    FileFacts {
        path: PathBuf::from(path),
        language: language.map(str::to_string),
        lines_of_code: 10,
        branch_count: 0,
        imports: Vec::new(),
        content: None,
        has_inline_tests,
    }
}
