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
fn ai_context_includes_context_risk_graph_sections() {
    let mut summary = make_summary(vec![]);
    summary.artifacts.context_graph_summary = Some(context_graph_summary());

    let output = render(&summary, &AiContextRenderOptions::default());

    assert!(output.contains("## Context Risk Graph"));
    assert!(output.contains("### Edit Order"));
    assert!(output.contains("### Blast Radius"));
    assert!(output.contains("### High-Context Files"));
    assert!(output.contains("### Verification Focus"));
    assert!(output.contains("src/core.rs"));
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
