use repopilot::audits::code_quality::complexity::{ComplexityAudit, count_branches};
use repopilot::audits::traits::FileAudit;
use repopilot::scan::config::ScanConfig;
use repopilot::scan::facts::FileFacts;
use std::path::PathBuf;

fn make_file(non_empty_lines: usize, branch_count: usize) -> FileFacts {
    FileFacts {
        path: PathBuf::from("src/lib.rs"),
        language: Some("Rust".to_string()),
        non_empty_lines,
        branch_count,
        imports: Vec::new(),
        content: None,
        has_inline_tests: false,
        in_executable_package: false,
        deferred_imports: Vec::new(),
    }
}

// ── ComplexityAudit findings ──────────────────────────────────────────────────

#[test]
fn no_finding_for_low_density() {
    // 100 LOC, 10 branches → density 100 < medium threshold 200
    let file = make_file(100, 10);
    let findings = ComplexityAudit.audit(&file, &ScanConfig::default());
    assert!(findings.is_empty());
}

#[test]
fn medium_finding_at_medium_threshold() {
    // 100 LOC, 25 branches → density 250 ≥ medium threshold 200
    let file = make_file(100, 25);
    let findings = ComplexityAudit.audit(&file, &ScanConfig::default());
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "code-quality.complex-file");
}

#[test]
fn high_finding_at_high_threshold() {
    // 100 LOC, 50 branches → density 500 ≥ high threshold 400
    let file = make_file(100, 50);
    let findings = ComplexityAudit.audit(&file, &ScanConfig::default());
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "code-quality.complex-file");
}

#[test]
fn tiny_dense_file_is_capped_at_medium_complexity() {
    // 12 LOC, 6 branches -> density 500, but tiny helpers should not be high severity.
    let file = make_file(12, 6);
    let findings = ComplexityAudit.audit(&file, &ScanConfig::default());
    assert_eq!(findings.len(), 1);
}

#[test]
fn high_severity_wins_over_medium() {
    // density exactly at high threshold must be High, not Medium
    let file = make_file(100, 40);
    let findings = ComplexityAudit.audit(&file, &ScanConfig::default());
    assert_eq!(findings.len(), 1);
}

#[test]
fn skips_files_under_ten_loc() {
    // 5 LOC → audit returns nothing regardless of branch count
    let file = make_file(5, 50);
    let findings = ComplexityAudit.audit(&file, &ScanConfig::default());
    assert!(findings.is_empty());
}

#[test]
fn skips_unsupported_language() {
    let file = FileFacts {
        path: PathBuf::from("README.md"),
        language: Some("Markdown".to_string()),
        non_empty_lines: 100,
        branch_count: 50,
        imports: Vec::new(),
        content: None,
        has_inline_tests: false,
        in_executable_package: false,
        deferred_imports: Vec::new(),
    };
    let findings = ComplexityAudit.audit(&file, &ScanConfig::default());
    assert!(findings.is_empty());
}

#[test]
fn config_override_raises_threshold() {
    // density 250 → would be Medium at default threshold 200, but not at threshold 300
    let file = make_file(100, 25);
    let config = ScanConfig {
        complexity_medium_threshold: 300,
        ..ScanConfig::default()
    };
    let findings = ComplexityAudit.audit(&file, &config);
    assert!(findings.is_empty());
}

#[test]
fn config_override_lowers_threshold() {
    // density 100 → no finding at default threshold 200, but Medium at threshold 50
    let file = make_file(100, 10);
    let config = ScanConfig {
        complexity_medium_threshold: 50,
        complexity_high_threshold: 400,
        ..ScanConfig::default()
    };
    let findings = ComplexityAudit.audit(&file, &config);
    assert_eq!(findings.len(), 1);
}

// ── count_branches unit tests ─────────────────────────────────────────────────

#[test]
fn counts_if_for_while_match() {
    let code = "if x { for y in z { while cond { match v { } } } }";
    assert_eq!(count_branches(code), 4);
}

#[test]
fn counts_else_keyword() {
    let code = "if x { } else { }";
    // "if " → 1, "else " → 1
    assert_eq!(count_branches(code), 2);
}

#[test]
fn does_not_count_else_inside_longer_word() {
    // "elsewhere" must not be counted as "else"
    let code = "let elsewhere = true;";
    assert_eq!(count_branches(code), 0);
}

#[test]
fn skips_comment_lines() {
    let code = "// if this were counted it would be a bug\nif real_condition {";
    assert_eq!(count_branches(code), 1);
}

#[test]
fn counts_logical_operators() {
    let code = "if a && b || c {";
    // "if " → 1, "&&" → 1, "||" → 1
    assert_eq!(count_branches(code), 3);
}
