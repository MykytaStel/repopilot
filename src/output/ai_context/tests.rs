use super::header::risk_level;
use super::{AiContextRenderOptions, AiFocusCategory, render};
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::scan::types::ScanSummary;
use std::path::PathBuf;

fn make_finding(
    rule_id: &str,
    title: &str,
    severity: Severity,
    category: FindingCategory,
    path: &str,
    line: usize,
) -> Finding {
    Finding {
        id: rule_id.to_string(),
        rule_id: rule_id.to_string(),
        recommendation: Finding::recommendation_for_rule_id(rule_id),
        title: title.to_string(),
        description: format!("Description for {title}"),
        category,
        severity,
        confidence: Default::default(),
        evidence: vec![Evidence {
            path: PathBuf::from(path),
            line_start: line,
            line_end: None,
            snippet: format!("// snippet for {title}"),
        }],
        workspace_package: None,
        docs_url: None,
        provenance: Default::default(),
        risk: Default::default(),
    }
}

fn make_summary(findings: Vec<Finding>) -> ScanSummary {
    ScanSummary {
        root_path: PathBuf::from("/my-project"),
        files_discovered: 0,
        files_analyzed: 42,
        non_empty_lines: 3000,
        directories_count: 10,
        languages: vec![],
        findings,
        ..Default::default()
    }
}

#[test]
fn renders_header_with_risk_high() {
    let findings = vec![make_finding(
        "security.secret",
        "Hardcoded secret",
        Severity::Critical,
        FindingCategory::Security,
        "src/auth.rs",
        42,
    )];
    let summary = make_summary(findings);
    let output = render(&summary, &AiContextRenderOptions::default());
    assert!(output.contains("# RepoPilot AI Context — my-project"));
    assert!(output.contains("🔴 HIGH"));
    assert!(output.contains("1 critical"));
}

#[test]
fn renders_no_header_when_flag_set() {
    let summary = make_summary(vec![]);
    let opts = AiContextRenderOptions {
        no_header: true,
        ..Default::default()
    };
    let output = render(&summary, &opts);
    assert!(!output.contains("# RepoPilot AI Context"));
}

#[test]
fn focus_filters_to_security_only() {
    let findings = vec![
        make_finding(
            "security.secret",
            "Hardcoded secret",
            Severity::Critical,
            FindingCategory::Security,
            "src/auth.rs",
            1,
        ),
        make_finding(
            "architecture.large-file",
            "Large file",
            Severity::Medium,
            FindingCategory::Architecture,
            "src/big.rs",
            1,
        ),
    ];
    let summary = make_summary(findings);
    let opts = AiContextRenderOptions {
        focus: Some(AiFocusCategory::Security),
        ..Default::default()
    };
    let output = render(&summary, &opts);
    assert!(output.contains("Security"));
    assert!(!output.contains("Architecture"));
}

#[test]
fn focus_quality_includes_testing_and_code_quality() {
    let findings = vec![
        make_finding(
            "code-quality.todo",
            "TODO marker",
            Severity::Low,
            FindingCategory::CodeQuality,
            "src/a.rs",
            1,
        ),
        make_finding(
            "testing.missing-tests",
            "Missing tests",
            Severity::Medium,
            FindingCategory::Testing,
            "src/b.rs",
            1,
        ),
        make_finding(
            "security.secret",
            "Secret",
            Severity::Critical,
            FindingCategory::Security,
            "src/c.rs",
            1,
        ),
    ];
    let summary = make_summary(findings);
    let opts = AiContextRenderOptions {
        focus: Some(AiFocusCategory::Quality),
        ..Default::default()
    };
    let output = render(&summary, &opts);
    assert!(output.contains("Code Quality") || output.contains("Testing"));
    assert!(!output.contains("Security"));
}

#[test]
fn finding_entries_include_context_confidence_and_fix() {
    let findings = vec![make_finding(
        "code-quality.long-function",
        "Long React component",
        Severity::Low,
        FindingCategory::CodeQuality,
        "src/Profile.tsx",
        12,
    )];
    let summary = make_summary(findings);
    let opts = AiContextRenderOptions {
        no_header: true,
        ..Default::default()
    };

    let output = render(&summary, &opts);

    assert!(output.contains("> **Confidence:** MEDIUM"));
    assert!(output.contains("> **Context:** Description for Long React component"));
    assert!(output.contains("> **Fix:**"));
}

#[test]
fn small_budget_renders_truncation_notice() {
    let findings: Vec<Finding> = (0..8)
        .map(|i| {
            make_finding(
                "security.secret",
                &format!("Hardcoded secret {i}"),
                Severity::High,
                FindingCategory::Security,
                &format!("src/auth_{i}.rs"),
                i + 1,
            )
        })
        .collect();
    let summary = make_summary(findings);
    let opts = AiContextRenderOptions {
        budget_tokens: 20,
        no_header: true,
        ..Default::default()
    };
    let output = render(&summary, &opts);
    assert!(output.contains("Output truncated to stay within token budget"));
}

#[test]
fn risk_level_moderate_for_one_high() {
    let findings = [make_finding(
        "arch.coupling",
        "High coupling",
        Severity::High,
        FindingCategory::Architecture,
        "src/a.rs",
        1,
    )];
    let refs: Vec<&Finding> = findings.iter().collect();
    assert_eq!(risk_level(&refs), "🟡 MODERATE");
}

#[test]
fn risk_level_elevated_for_three_high() {
    let findings: Vec<Finding> = (0..3)
        .map(|i| {
            make_finding(
                "arch.coupling",
                "High coupling",
                Severity::High,
                FindingCategory::Architecture,
                "src/a.rs",
                i,
            )
        })
        .collect();
    let refs: Vec<&Finding> = findings.iter().collect();
    assert_eq!(risk_level(&refs), "🟠 ELEVATED");
}

#[test]
fn risk_level_low_for_no_high() {
    let findings = [make_finding(
        "code.todo",
        "TODO",
        Severity::Low,
        FindingCategory::CodeQuality,
        "src/a.rs",
        1,
    )];
    let refs: Vec<&Finding> = findings.iter().collect();
    assert_eq!(risk_level(&refs), "🟢 LOW");
}

#[test]
fn risk_level_moderate_for_many_medium_findings() {
    let findings: Vec<Finding> = (0..10)
        .map(|index| {
            make_finding(
                "architecture.large-file",
                "Large file",
                Severity::Medium,
                FindingCategory::Architecture,
                "src/a.rs",
                index + 1,
            )
        })
        .collect();
    let refs: Vec<&Finding> = findings.iter().collect();
    assert_eq!(risk_level(&refs), "🟡 MODERATE");
}

#[test]
fn token_estimate_in_footer() {
    let summary = make_summary(vec![]);
    let output = render(&summary, &AiContextRenderOptions::default());
    assert!(output.contains("tokens"));
    assert!(output.contains("budget: 4096"));
}

#[test]
fn top_recommendations_omitted_when_no_high_findings() {
    let findings = vec![make_finding(
        "code.todo",
        "TODO marker",
        Severity::Low,
        FindingCategory::CodeQuality,
        "src/a.rs",
        1,
    )];
    let summary = make_summary(findings);
    let output = render(&summary, &AiContextRenderOptions::default());
    assert!(!output.contains("## Top Recommendations"));
}

#[test]
fn top_recommendations_include_medium_clusters_when_no_high_findings() {
    let findings = vec![
        make_finding(
            "architecture.large-file",
            "Large file detected",
            Severity::Medium,
            FindingCategory::Architecture,
            "src/a.rs",
            1,
        ),
        make_finding(
            "architecture.large-file",
            "Large file detected",
            Severity::Medium,
            FindingCategory::Architecture,
            "src/b.rs",
            1,
        ),
    ];
    let summary = make_summary(findings);
    let output = render(&summary, &AiContextRenderOptions::default());
    assert!(output.contains("## Top Recommendations"));
    assert!(output.contains("MEDIUM 2 finding(s)"));
}

#[test]
fn top_recommendations_shown_for_high_findings() {
    let findings = vec![make_finding(
        "security.secret",
        "Hardcoded secret",
        Severity::High,
        FindingCategory::Security,
        "src/a.rs",
        5,
    )];
    let summary = make_summary(findings);
    let output = render(&summary, &AiContextRenderOptions::default());
    assert!(output.contains("## Top Recommendations"));
}

#[test]
fn empty_scan_renders_without_panic() {
    let summary = make_summary(vec![]);
    let output = render(&summary, &AiContextRenderOptions::default());
    assert!(output.contains("RepoPilot AI Context"));
    assert!(output.contains("0 findings"));
}

#[test]
fn ai_context_category_from_str() {
    assert_eq!("security".parse(), Ok(AiFocusCategory::Security));
    assert_eq!("arch".parse(), Ok(AiFocusCategory::Architecture));
    assert_eq!("quality".parse(), Ok(AiFocusCategory::Quality));
    assert_eq!("framework".parse(), Ok(AiFocusCategory::Framework));
    assert_eq!("all".parse(), Ok(AiFocusCategory::All));
    assert_eq!("unknown".parse::<AiFocusCategory>(), Err(()));
}
