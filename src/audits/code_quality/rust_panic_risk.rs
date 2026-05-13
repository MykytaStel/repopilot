use crate::audits::context::{FileRole, LanguageKind, RuntimeKind, classify_file};
use crate::audits::traits::FileAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::knowledge::decision::decide_for_audit_context;
use crate::scan::config::ScanConfig;
use crate::scan::facts::FileFacts;
use crate::scan::path_classification::is_low_signal_audit_path;

const RULE_ID: &str = "language.rust.panic-risk";

pub struct RustPanicRiskAudit;

impl FileAudit for RustPanicRiskAudit {
    fn audit(&self, file: &FileFacts, _config: &ScanConfig) -> Vec<Finding> {
        if is_low_signal_audit_path(&file.path) {
            return vec![];
        }

        let context = classify_file(file);

        if context.language != LanguageKind::Rust {
            return vec![];
        }

        let Some(content) = file.content.as_deref() else {
            return vec![];
        };

        let mut findings = Vec::new();
        let mut in_block_comment = false;

        for (index, line) in content.lines().enumerate() {
            let line_number = index + 1;
            let trimmed = line.trim();

            if trimmed.is_empty() {
                continue;
            }

            let sanitized = sanitize_rust_line(line, &mut in_block_comment);
            let Some(pattern) = detect_pattern(sanitized.trim()) else {
                continue;
            };

            let decision = decide_for_audit_context(
                RULE_ID,
                &context,
                pattern.base_severity(),
                Some(pattern.signal()),
            );

            if decision.is_suppressed() {
                continue;
            }

            findings.push(build_finding(
                file,
                line_number,
                trimmed,
                pattern,
                &context,
                decision.severity,
            ));
        }

        findings
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RustPanicPattern {
    Unwrap,
    Expect,
    Panic,
    Todo,
    Unimplemented,
}

impl RustPanicPattern {
    fn label(self) -> &'static str {
        match self {
            RustPanicPattern::Unwrap => "unwrap()",
            RustPanicPattern::Expect => "expect()",
            RustPanicPattern::Panic => "panic!",
            RustPanicPattern::Todo => "todo!",
            RustPanicPattern::Unimplemented => "unimplemented!",
        }
    }

    fn signal(self) -> &'static str {
        match self {
            RustPanicPattern::Unwrap => "rust.unwrap",
            RustPanicPattern::Expect => "rust.expect",
            RustPanicPattern::Panic => "rust.panic",
            RustPanicPattern::Todo => "rust.todo",
            RustPanicPattern::Unimplemented => "rust.unimplemented",
        }
    }

    fn base_severity(self) -> Severity {
        match self {
            RustPanicPattern::Todo | RustPanicPattern::Unimplemented => Severity::High,
            RustPanicPattern::Unwrap | RustPanicPattern::Expect | RustPanicPattern::Panic => {
                Severity::Medium
            }
        }
    }
}

fn detect_pattern(trimmed: &str) -> Option<RustPanicPattern> {
    if trimmed.contains("todo!(") {
        return Some(RustPanicPattern::Todo);
    }

    if trimmed.contains("unimplemented!(") {
        return Some(RustPanicPattern::Unimplemented);
    }

    if trimmed.contains("panic!(") {
        return Some(RustPanicPattern::Panic);
    }

    if trimmed.contains(".unwrap()") {
        return Some(RustPanicPattern::Unwrap);
    }

    if trimmed.contains(".expect(") {
        return Some(RustPanicPattern::Expect);
    }

    None
}

fn sanitize_rust_line(line: &str, in_block_comment: &mut bool) -> String {
    // Lightweight scanner only; raw strings and nested block comments are a
    // future parser-level improvement, not a dependency for this audit.
    let mut output = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    let mut in_string = false;
    let mut in_char = false;
    let mut escaped = false;

    while let Some(character) = chars.next() {
        if *in_block_comment {
            if character == '*' && chars.peek() == Some(&'/') {
                chars.next();
                *in_block_comment = false;
                output.push(' ');
                output.push(' ');
            } else {
                output.push(' ');
            }
            continue;
        }

        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            output.push(' ');
            continue;
        }

        if in_char {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '\'' {
                in_char = false;
            }
            output.push(' ');
            continue;
        }

        if character == '/' && chars.peek() == Some(&'/') {
            break;
        }

        if character == '/' && chars.peek() == Some(&'*') {
            chars.next();
            *in_block_comment = true;
            output.push(' ');
            output.push(' ');
            continue;
        }

        if character == '"' {
            in_string = true;
            output.push(' ');
            continue;
        }

        if character == '\'' {
            in_char = true;
            output.push(' ');
            continue;
        }

        output.push(character);
    }

    output
}

fn build_finding(
    file: &FileFacts,
    line_number: usize,
    snippet: &str,
    pattern: RustPanicPattern,
    context: &crate::audits::context::AuditContext,
    severity: Severity,
) -> Finding {
    let context_label = context_label(context);
    let recommendation = recommendation_for(context, pattern);

    Finding {
        id: String::new(),
        rule_id: RULE_ID.to_string(),
        title: format!("Risky Rust {} usage in {}", pattern.label(), context_label),
        description: format!(
            "Rust `{}` was found in {}. {}",
            pattern.label(),
            context_label,
            recommendation
        ),
        category: FindingCategory::CodeQuality,
        severity,
        confidence: Default::default(),
        evidence: vec![Evidence {
            path: file.path.clone(),
            line_start: line_number,
            line_end: None,
            snippet: snippet.to_string(),
        }],
        workspace_package: None,
        docs_url: None,
    }
}

fn context_label(context: &crate::audits::context::AuditContext) -> &'static str {
    if context.is_test {
        return "Rust test code";
    }

    if context.has_runtime(RuntimeKind::RustCli) {
        return "Rust CLI boundary code";
    }

    if context.has_role(FileRole::Domain) {
        return "Rust domain code";
    }

    if context.has_runtime(RuntimeKind::RustLibrary) {
        return "Rust library code";
    }

    "Rust production code"
}

fn recommendation_for(
    context: &crate::audits::context::AuditContext,
    pattern: RustPanicPattern,
) -> &'static str {
    if context.is_test {
        return "Panic-style helpers in tests can be acceptable, but keep them out of reusable test utilities when possible.";
    }

    match pattern {
        RustPanicPattern::Unwrap | RustPanicPattern::Expect => {
            if context.has_runtime(RuntimeKind::RustCli) {
                "At CLI boundaries this may be acceptable for prototype code, but prefer returning a user-friendly error with context."
            } else {
                "Prefer propagating errors with `?`, returning `Result`, or converting the failure into a domain-specific error."
            }
        }
        RustPanicPattern::Panic => {
            "Avoid panics in reusable production code. Prefer typed errors, validation, or explicit fallback behavior."
        }
        RustPanicPattern::Todo | RustPanicPattern::Unimplemented => {
            "Replace placeholder macros before release or guard them behind test-only code paths."
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert!(findings[0].title.contains("unwrap()"));
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
        assert!(findings[0].title.contains("expect()"));
        assert!(findings[0].description.contains("Rust domain code"));
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
            lines_of_code: content.lines().count(),
            branch_count: 0,
            imports: Vec::new(),
            content: Some(content.to_string()),
            has_inline_tests,
        }
    }
}
