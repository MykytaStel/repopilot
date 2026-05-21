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
