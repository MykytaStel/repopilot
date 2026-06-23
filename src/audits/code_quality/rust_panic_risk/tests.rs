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
fn ignores_literal_regex_new_unwrap_in_production_code() {
    // A literal regex is infallible by construction; the `.unwrap()` form (no
    // "valid regex" message for the text heuristic to match) must still be
    // skipped via the structural AST check.
    let file = facts(
        "src/scraper/extract.rs",
        "let re = Regex::new(r\"\\d{4}\").unwrap();",
        false,
    );

    let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());

    assert!(findings.is_empty());
}

#[test]
fn ignores_literal_selector_parse_expect_in_production_code() {
    let file = facts(
        "src/scraper/extract.rs",
        "let sel = Selector::parse(\"div.price\").expect(\"selector\");",
        false,
    );

    let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());

    assert!(findings.is_empty());
}

#[test]
fn ignores_multiline_literal_regex_new_unwrap() {
    let file = facts(
        "src/scraper/extract.rs",
        "let re = Regex::new(\n    r\"^[a-z]+$\",\n).unwrap();",
        false,
    );

    let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());

    assert!(findings.is_empty());
}

#[test]
fn keeps_dynamic_regex_new_unwrap_as_risk() {
    // A regex built from a runtime value can genuinely fail to compile, so it
    // stays a panic risk — only string-literal patterns are exempt.
    let file = facts(
        "src/scraper/extract.rs",
        "let re = Regex::new(&pattern).unwrap();",
        false,
    );

    let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());

    assert_eq!(findings.len(), 1);
    assert!(findings[0].title.contains("unwrap()"));
}

#[test]
fn downgrades_literal_parse_unwrap_to_low() {
    // `"literal".parse().unwrap()` (e.g. a default color/spec table) parses a
    // programmer-controlled string. It can still panic (`"999".parse::<u8>()`),
    // but as a deterministic bug caught on the first run — so it is downgraded to
    // Low (hidden in default, kept in strict), NOT removed, rather than escalated
    // to a visible High by the `.parse(` external signal.
    let file = facts(
        "src/printer/color.rs",
        "let spec = \"path:fg:magenta\".parse().unwrap();",
        false,
    );

    let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Low);
}

#[test]
fn downgrades_literal_parse_unwrap_in_macro_to_low() {
    // The same call inside a `vec![…]` macro, whose body is an unparsed token
    // tree the AST check cannot see — the text fallback downgrades it to Low.
    let file = facts(
        "src/printer/color.rs",
        "    \"match:style:bold\".parse().unwrap(),",
        false,
    );

    let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Low);
}

#[test]
fn keeps_typed_literal_parse_unwrap_visible() {
    // An explicit `.parse::<T>()` is a deliberate typed conversion, more likely
    // to be a real risk worth surfacing, so it is NOT downgraded — only the
    // type-inferred `.parse()` form is.
    let file = facts(
        "src/config/load.rs",
        "let port = \"999\".parse::<u8>().unwrap();",
        false,
    );

    let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());

    assert_eq!(findings.len(), 1);
    assert!(findings[0].severity >= Severity::Medium);
}

#[test]
fn keeps_dynamic_parse_unwrap_as_risk() {
    // Parsing a runtime value (external input) can genuinely fail, so it stays a
    // visible High — only a `.parse()` on a string literal is downgraded.
    let file = facts(
        "src/config/load.rs",
        "let port: u16 = raw.parse().unwrap();",
        false,
    );

    let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::High);
}

#[test]
fn downgrades_panic_in_rust_test_support_module() {
    // `testutil.rs` is test infrastructure compiled in normal builds; its
    // `panic!` is assertion plumbing, not production risk. The test role
    // downgrades it to Low — hidden in the default profile, kept in strict — the
    // same treatment `panic!` already gets under `tests/`.
    let file = facts(
        "crates/searcher/src/testutil.rs",
        "pub fn run() { panic!(\"test configuration produced nothing\"); }",
        false,
    );

    let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Low);
}

#[test]
fn suppresses_unwrap_in_rust_test_support_module() {
    // An `unwrap` in test-support code is assertion setup; the test role
    // suppresses it entirely, as it does for `tests/` files.
    let file = facts(
        "crates/searcher/src/testutil.rs",
        "pub fn first(v: &[u8]) -> u8 { *v.first().unwrap() }",
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
fn mutex_lock_unwrap_named_db_is_not_escalated_to_high() {
    // `self.db.lock().unwrap()` is a poisoned-mutex panic, not an external
    // database failure. The `db.` substring must not escalate it to High.
    let file = facts(
        "src/state.rs",
        "let mut guard = self.db.lock().unwrap();",
        false,
    );

    let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Medium);
}

#[test]
fn serde_json_serialization_unwrap_is_not_escalated_to_high() {
    // `serde_json::to_string(&value).unwrap()` serializes an owned, in-memory
    // value to JSON — an in-process, effectively-infallible operation. The
    // `serde_json`/`json` external signals must not escalate it to a visible
    // High the way a genuine deserialization is.
    for line in [
        "changes: Some(serde_json::to_string(&changes).unwrap()),",
        "before.insert(id.clone(), serde_json::to_value(&snapshot).unwrap());",
    ] {
        let file = facts("src/analysis/merge_service.rs", line, false);
        let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());
        assert_eq!(findings.len(), 1, "{line}");
        assert_eq!(findings[0].severity, Severity::Medium, "{line}");
    }
}

#[test]
fn serde_json_deserialization_unwrap_stays_high() {
    // Parsing untrusted external bytes back into a value genuinely fails on
    // malformed input, so it remains a visible High external-failure path.
    let file = facts(
        "src/api/handler.rs",
        "let parsed: Payload = serde_json::from_str(&body).unwrap();",
        false,
    );

    let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::High);
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
        in_executable_package: false,
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
