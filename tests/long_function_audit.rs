use repopilot::audits::code_quality::long_function::LongFunctionAudit;
use repopilot::audits::traits::FileAudit;
use repopilot::findings::types::Severity;
use repopilot::scan::config::ScanConfig;
use repopilot::scan::facts::FileFacts;
use std::path::PathBuf;

fn make_file(language: &str, content: &str) -> FileFacts {
    FileFacts {
        path: PathBuf::from("src/lib.rs"),
        language: Some(language.to_string()),
        lines_of_code: content.lines().count(),
        branch_count: 0,
        imports: Vec::new(),
        content: content.to_string(),
    }
}

fn config_with_threshold(threshold: usize) -> ScanConfig {
    ScanConfig {
        long_function_loc_threshold: threshold,
        ..ScanConfig::default()
    }
}

// ── Rust ──────────────────────────────────────────────────────────────────────

#[test]
fn rust_function_under_threshold_produces_no_finding() {
    let content = "fn short() {\n    let x = 1;\n}\n";
    let file = make_file("Rust", content);
    let findings = LongFunctionAudit.audit(&file, &config_with_threshold(50));
    assert!(findings.is_empty());
}

#[test]
fn rust_function_over_threshold_produces_finding() {
    let body: String = (0..60).map(|i| format!("    let _{i} = {i};\n")).collect();
    let content = format!("fn long_fn() {{\n{body}}}\n");
    let file = make_file("Rust", &content);
    let findings = LongFunctionAudit.audit(&file, &config_with_threshold(50));
    assert_eq!(
        findings.len(),
        1,
        "expected one finding for a long Rust function"
    );
    assert_eq!(findings[0].rule_id, "code-quality.long-function");
    assert_eq!(findings[0].severity, Severity::Medium);
    assert!(findings[0].title.contains("long_fn"));
}

#[test]
fn rust_two_functions_only_long_one_flagged() {
    let short_body: String = (0..10).map(|i| format!("    let _{i} = {i};\n")).collect();
    let long_body: String = (0..60).map(|i| format!("    let _{i} = {i};\n")).collect();
    let content = format!("fn short_fn() {{\n{short_body}}}\nfn long_fn() {{\n{long_body}}}\n");
    let file = make_file("Rust", &content);
    let findings = LongFunctionAudit.audit(&file, &config_with_threshold(50));
    assert_eq!(findings.len(), 1);
    assert!(findings[0].title.contains("long_fn"));
}

// ── Python ────────────────────────────────────────────────────────────────────

#[test]
fn python_function_under_threshold_no_finding() {
    let content = "def short():\n    return 1\n";
    let file = make_file("Python", content);
    let findings = LongFunctionAudit.audit(&file, &config_with_threshold(50));
    assert!(findings.is_empty());
}

#[test]
fn python_function_over_threshold_produces_finding() {
    let body: String = (0..60).map(|i| format!("    x_{i} = {i}\n")).collect();
    let content = format!("def big_fn():\n{body}\ndef next_fn():\n    pass\n");
    let file = make_file("Python", &content);
    let findings = LongFunctionAudit.audit(&file, &config_with_threshold(50));
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "code-quality.long-function");
    assert!(findings[0].title.contains("big_fn"));
}

#[test]
fn python_function_extending_to_eof_is_detected() {
    // Function body goes to the very end of the file — exercises the EOF flush path.
    let body: String = (0..60).map(|i| format!("    x_{i} = {i}\n")).collect();
    let content = format!("def eof_fn():\n{body}");
    let file = make_file("Python", &content);
    let findings = LongFunctionAudit.audit(&file, &config_with_threshold(50));
    assert_eq!(
        findings.len(),
        1,
        "EOF-terminated Python function should be detected"
    );
    assert!(findings[0].title.contains("eof_fn"));
}

// ── Unsupported language ──────────────────────────────────────────────────────

#[test]
fn unsupported_language_produces_no_finding() {
    let content = "def foo():\n    pass\n";
    let file = make_file("Ruby", content);
    let findings = LongFunctionAudit.audit(&file, &config_with_threshold(1));
    assert!(findings.is_empty());
}
