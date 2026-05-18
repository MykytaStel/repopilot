use repopilot::audits::code_quality::long_function::LongFunctionAudit;
use repopilot::audits::traits::FileAudit;
use repopilot::findings::types::{Confidence, Severity};
use repopilot::scan::config::ScanConfig;
use repopilot::scan::facts::FileFacts;
use std::path::PathBuf;

fn make_file(language: &str, content: &str) -> FileFacts {
    make_file_at("src/lib.rs", language, content)
}

fn make_file_at(path: &str, language: &str, content: &str) -> FileFacts {
    FileFacts {
        path: PathBuf::from(path),
        language: Some(language.to_string()),
        non_empty_lines: content.lines().count(),
        branch_count: 0,
        imports: Vec::new(),
        content: Some(content.to_string()),
        has_inline_tests: false,
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
    assert_eq!(findings[0].confidence, Confidence::High);
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

#[test]
fn rust_long_function_in_test_path_is_skipped() {
    let body: String = (0..60).map(|i| format!("    let _{i} = {i};\n")).collect();
    let content = format!("fn long_test_helper() {{\n{body}}}\n");
    let file = make_file_at("tests/long_function_audit.rs", "Rust", &content);
    let findings = LongFunctionAudit.audit(&file, &config_with_threshold(50));

    assert!(
        findings.is_empty(),
        "test files should not create medium long-function noise"
    );
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

// ── Java ──────────────────────────────────────────────────────────────────────

#[test]
fn java_method_over_threshold_produces_finding() {
    let body: String = (0..60)
        .map(|i| format!("        int x{i} = {i};\n"))
        .collect();
    let content =
        format!("public class Foo {{\n    public void bigMethod() {{\n{body}    }}\n}}\n");
    let file = make_file("Java", &content);
    let findings = LongFunctionAudit.audit(&file, &config_with_threshold(50));
    assert_eq!(
        findings.len(),
        1,
        "expected one finding for a long Java method"
    );
    assert!(
        findings[0].title.contains("bigMethod"),
        "{:?}",
        findings[0].title
    );
}

#[test]
fn java_constructor_flagged() {
    let body: String = (0..60)
        .map(|i| format!("        this.x{i} = {i};\n"))
        .collect();
    let content = format!("public class Foo {{\n    public Foo() {{\n{body}    }}\n}}\n");
    let file = make_file("Java", &content);
    let findings = LongFunctionAudit.audit(&file, &config_with_threshold(50));
    assert_eq!(
        findings.len(),
        1,
        "constructor should be detected as a long function"
    );
}

#[test]
fn java_if_block_not_flagged_as_function() {
    let body: String = (0..60)
        .map(|i| format!("        int x{i} = {i};\n"))
        .collect();
    let content = format!(
        "public class Foo {{\n    void m() {{\n        if (true) {{\n{body}        }}\n    }}\n}}\n"
    );
    let file = make_file("Java", &content);
    let findings = LongFunctionAudit.audit(&file, &config_with_threshold(50));
    // Only the outer method `m` may be flagged if long enough, but the if-block must not add a separate finding
    assert!(
        findings.iter().all(|f| !f.title.contains("<anonymous>")),
        "if-block must not produce a function finding: {:?}",
        findings
    );
}

// ── Kotlin ────────────────────────────────────────────────────────────────────

#[test]
fn kotlin_fun_over_threshold_produces_finding() {
    let body: String = (0..60).map(|i| format!("    val x{i} = {i}\n")).collect();
    let content = format!("fun bigFun() {{\n{body}}}\n");
    let file = make_file("Kotlin", &content);
    let findings = LongFunctionAudit.audit(&file, &config_with_threshold(50));
    assert_eq!(
        findings.len(),
        1,
        "expected one finding for a long Kotlin function"
    );
    assert!(
        findings[0].title.contains("bigFun"),
        "{:?}",
        findings[0].title
    );
}

#[test]
fn kotlin_suspend_fun_detected() {
    let body: String = (0..60).map(|i| format!("    val x{i} = {i}\n")).collect();
    let content = format!("suspend fun networkCall() {{\n{body}}}\n");
    let file = make_file("Kotlin", &content);
    let findings = LongFunctionAudit.audit(&file, &config_with_threshold(50));
    assert_eq!(
        findings.len(),
        1,
        "suspend modifier must not block detection"
    );
    assert!(
        findings[0].title.contains("networkCall"),
        "{:?}",
        findings[0].title
    );
}

#[test]
fn kotlin_private_override_fun_detected() {
    let body: String = (0..60).map(|i| format!("    val x{i} = {i}\n")).collect();
    let content = format!("private override fun render() {{\n{body}}}\n");
    let file = make_file("Kotlin", &content);
    let findings = LongFunctionAudit.audit(&file, &config_with_threshold(50));
    assert_eq!(
        findings.len(),
        1,
        "private override modifiers must be stripped"
    );
    assert!(
        findings[0].title.contains("render"),
        "{:?}",
        findings[0].title
    );
}

// ── Unsupported language ──────────────────────────────────────────────────────

#[test]
fn unsupported_language_produces_no_finding() {
    let content = "def foo():\n    pass\n";
    let file = make_file("Ruby", content);
    let findings = LongFunctionAudit.audit(&file, &config_with_threshold(1));
    assert!(findings.is_empty());
}
