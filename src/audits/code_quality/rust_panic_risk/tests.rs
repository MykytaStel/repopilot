use super::*;
use crate::findings::types::{Confidence, Severity};
use crate::scan::facts::FileFacts;
use std::path::PathBuf;

#[test]
fn ignores_unwrap_in_rust_tests() {
    let file = facts(
        "tests/parser_test.rs",
        "let value = parse().unwrap();",
        true,
    );

    let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());

    assert!(findings.is_empty());
}

#[test]
fn reports_unwrap_in_rust_library_code() {
    let file = facts(
        "src/domain/parser.rs",
        "let value = parse().unwrap();",
        false,
    );

    let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, RULE_ID);
    assert_eq!(findings[0].severity, Severity::Medium);
    assert_eq!(findings[0].confidence, Confidence::High);
    assert!(findings[0].title.contains("unwrap()"));
    assert!(
        findings[0]
            .description
            .contains("confidence is HIGH because")
    );
    assert!(findings[0].recommendation.contains("Return `Result`"));
}

#[test]
fn reports_panic_in_domain_code_as_high() {
    let file = facts(
        "src/domain/user.rs",
        "panic!(\"invalid user state\");",
        false,
    );

    let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::High);
    assert!(findings[0].title.contains("panic!"));
}

#[test]
fn reports_todo_in_production_code_as_high() {
    let file = facts(
        "src/service.rs",
        "todo!(\"implement payment flow\");",
        false,
    );

    let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::High);
    assert!(findings[0].title.contains("todo!"));
}

#[test]
fn lowers_unwrap_severity_in_rust_cli_boundary() {
    let file = facts("src/main.rs", "let config = load_config().unwrap();", false);

    let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Low);
    assert_eq!(findings[0].confidence, Confidence::Low);
    assert!(findings[0].description.contains("CLI boundary"));
}

#[test]
fn ignores_commented_panic_patterns() {
    let file = facts(
        "src/lib.rs",
        "// let value = parse().unwrap();\n/// panic!(\"example\");\n/*\n * value.unwrap()\n */\n// panic!(\"old code\");",
        false,
    );

    let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());

    assert!(findings.is_empty());
}

#[test]
fn ignores_string_literal_panic_patterns() {
    let file = facts(
        "src/lib.rs",
        "let text = \"value.unwrap() and panic!(\\\"example\\\")\";",
        false,
    );

    let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());

    assert!(findings.is_empty());
}

#[test]
fn does_not_report_functional_iterator_pipeline_without_panic_risk() {
    let file = facts(
        "src/domain/users.rs",
        "let names = users\n    .iter()\n    .filter(|user| user.is_active)\n    .map(|user| user.name.clone())\n    .collect::<Vec<_>>();\n",
        false,
    );

    let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());

    assert!(findings.is_empty());
}

#[test]
fn reports_expect_in_domain_code() {
    let file = facts(
        "src/domain/parser.rs",
        "let value = parse().expect(\"valid domain input\");",
        false,
    );

    let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Medium);
    assert_eq!(findings[0].confidence, Confidence::High);
    assert!(findings[0].title.contains("expect()"));
    assert!(findings[0].description.contains("Rust domain code"));
}

#[test]
fn reports_unwrap_err_and_expect_err_in_domain_code() {
    let file = facts(
        "src/domain/parser.rs",
        "let error = parse().unwrap_err();\nlet error = parse().expect_err(\"invalid input should fail\");",
        false,
    );

    let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());

    assert_eq!(findings.len(), 2);
    assert!(findings[0].title.contains("unwrap_err()"));
    assert!(findings[1].title.contains("expect_err()"));
}

#[test]
fn ignores_valid_regex_expect_in_production_code() {
    let file = facts(
        "src/domain/parser.rs",
        "let matcher = Regex::new(r\"^[a-z]+$\").expect(\"valid parser regex\");",
        false,
    );

    let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());

    assert!(findings.is_empty());
}

#[test]
fn ignores_mutex_poison_invariant_expect() {
    let file = facts(
        "src/state.rs",
        "let guard = cache.lock().expect(\"cache mutex should not be poisoned\");",
        false,
    );

    let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());

    assert!(findings.is_empty());
}

#[test]
fn upgrades_external_parse_unwrap_to_high() {
    let file = facts(
        "src/domain/parser.rs",
        "let value = raw.parse::<u64>().unwrap();",
        false,
    );

    let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::High);
    assert_eq!(findings[0].confidence, Confidence::High);
}

#[test]
fn reports_unimplemented_in_production_code() {
    let file = facts("src/lib.rs", "unimplemented!(\"missing adapter\");", false);

    let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::High);
    assert!(findings[0].title.contains("unimplemented!"));
}

#[test]
fn does_not_run_on_non_rust_files() {
    let mut file = facts("src/app.ts", "panic!(\"not rust\");", false);
    file.language = Some("TypeScript".to_string());

    let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());

    assert!(findings.is_empty());
}

#[test]
fn does_not_run_without_file_content() {
    let mut file = facts("src/lib.rs", "panic!(\"missing content\");", false);
    file.content = None;

    let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());

    assert!(findings.is_empty());
}
fn facts(path: &str, content: &str, has_inline_tests: bool) -> FileFacts {
    FileFacts {
        path: PathBuf::from(path),
        language: Some("Rust".to_string()),
        non_empty_lines: content.lines().count(),
        branch_count: 0,
        imports: Vec::new(),
        content: Some(content.to_string()),
        has_inline_tests,
    }
}

#[test]
fn ignores_unwrap_inside_inline_cfg_test_module() {
    // `unwrap()`/`panic!` inside an inline `#[cfg(test)]` block of a production
    // file is test code, not a production panic risk: only the production
    // `unwrap()` on the first line should be reported.
    let file = facts(
        "src/domain/parser.rs",
        "fn run() { let v = parse().unwrap(); }\n\
         #[cfg(test)]\n\
         mod tests {\n\
             #[test]\n\
             fn works() { let v = parse().unwrap(); }\n\
         }\n",
        true,
    );

    let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());

    assert_eq!(
        findings.len(),
        1,
        "only the production unwrap should report"
    );
    assert_eq!(findings[0].evidence[0].line_start, 1);
}

#[test]
fn falls_back_to_line_scanner_when_no_parsed_tree() {
    use crate::analysis::parse::ParsedFile;
    use crate::rules::SignalSource;

    let file = facts(
        "src/domain/parser.rs",
        "let value = parse().unwrap();",
        false,
    );

    // Create a ParsedFile with None for language label to force tree() to return None
    let parsed = ParsedFile::new("let value = parse().unwrap();", None);

    let findings = RustPanicRiskAudit.audit_parsed(&file, &parsed, &ScanConfig::default());

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, RULE_ID);
    assert_eq!(
        findings[0].provenance.signal_source,
        SignalSource::TextHeuristic
    );
}
