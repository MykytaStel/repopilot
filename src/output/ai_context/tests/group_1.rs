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
