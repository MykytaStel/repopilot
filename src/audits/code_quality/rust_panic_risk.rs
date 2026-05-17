use crate::audits::code_quality::sanitize::sanitize_c_style;
use crate::audits::context::{FileRole, LanguageKind, RuntimeKind, classify_file};
use crate::audits::traits::FileAudit;
use crate::findings::types::{Confidence, Evidence, Finding, FindingCategory, Severity};
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
        let mut pending_render_write = false;

        for (index, line) in content.lines().enumerate() {
            let line_number = index + 1;
            let trimmed = line.trim();

            if trimmed.is_empty() {
                continue;
            }

            let sanitized = sanitize_c_style(line, &mut in_block_comment);
            let sanitized = sanitized.trim();
            if is_infallible_render_write_start(&file.path, sanitized) {
                pending_render_write = true;
            }

            let Some(pattern) = detect_pattern(sanitized) else {
                if sanitized.ends_with(';') {
                    pending_render_write = false;
                }
                continue;
            };

            if pending_render_write && is_infallible_render_write_result_unwrap(pattern, sanitized)
            {
                if sanitized.ends_with(';') {
                    pending_render_write = false;
                }
                continue;
            }

            if should_ignore_contextual_panic_pattern(pattern, trimmed) {
                if sanitized.ends_with(';') {
                    pending_render_write = false;
                }
                continue;
            }

            let decision = decide_for_audit_context(
                RULE_ID,
                &context,
                pattern.base_severity(),
                Some(pattern.signal()),
            );

            if decision.is_suppressed() {
                continue;
            }

            let severity = if is_external_failure_path(pattern, sanitized) && !context.is_test {
                decision.severity.max(Severity::High)
            } else {
                decision.severity
            };

            findings.push(build_finding(
                file,
                line_number,
                trimmed,
                pattern,
                &context,
                severity,
            ));

            if sanitized.ends_with(';') {
                pending_render_write = false;
            }
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

fn should_ignore_contextual_panic_pattern(pattern: RustPanicPattern, raw_trimmed: &str) -> bool {
    if pattern != RustPanicPattern::Expect {
        return false;
    }

    let lower = raw_trimmed.to_lowercase();
    (lower.contains("valid") && lower.contains("regex") && lower.contains(".expect("))
        || (lower.contains("mutex") && lower.contains("poison") && lower.contains(".expect("))
}

fn is_external_failure_path(pattern: RustPanicPattern, sanitized: &str) -> bool {
    if !matches!(
        pattern,
        RustPanicPattern::Unwrap | RustPanicPattern::Expect | RustPanicPattern::Panic
    ) {
        return false;
    }

    let lower = sanitized.to_lowercase();
    const EXTERNAL_SIGNALS: &[&str] = &[
        ".parse(",
        ".parse::<",
        "from_str(",
        "from_slice(",
        "serde_json",
        "json",
        "env::var",
        "std::env",
        "var_os(",
        "fs::",
        "std::fs",
        "file::open",
        "read_to_string",
        ".read_",
        ".recv",
        ".send",
        "request",
        "reqwest",
        "hyper",
        "axum",
        "headers",
        "query",
        "params",
        "body",
        "sqlx",
        "diesel",
        "postgres",
        "mysql",
        "redis",
        "database",
        "db.",
        "pool.",
        "conn.",
        "tcpstream",
        "udp",
        "socket",
    ];

    EXTERNAL_SIGNALS.iter().any(|signal| lower.contains(signal))
}

fn is_infallible_render_write_start(path: &std::path::Path, trimmed: &str) -> bool {
    is_report_renderer_path(path)
        && (trimmed.starts_with("writeln!(") || trimmed.starts_with("write!("))
}

fn is_report_renderer_path(path: &std::path::Path) -> bool {
    let mut previous = None;
    for component in path
        .components()
        .filter_map(|component| component.as_os_str().to_str())
    {
        if previous == Some("src") && component == "output" {
            return true;
        }
        previous = Some(component);
    }
    false
}

fn is_infallible_render_write_result_unwrap(pattern: RustPanicPattern, trimmed: &str) -> bool {
    match pattern {
        RustPanicPattern::Unwrap => {
            trimmed == ".unwrap();"
                || (is_infallible_render_write_line(trimmed) && trimmed.ends_with(".unwrap();"))
        }
        RustPanicPattern::Expect => {
            trimmed.starts_with(".expect(")
                || (is_infallible_render_write_line(trimmed) && trimmed.contains(").expect("))
        }
        RustPanicPattern::Panic | RustPanicPattern::Todo | RustPanicPattern::Unimplemented => false,
    }
}

fn is_infallible_render_write_line(trimmed: &str) -> bool {
    trimmed.starts_with("writeln!(") || trimmed.starts_with("write!(")
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
    let confidence = confidence_for(context, pattern, severity);
    let confidence_reason = confidence_reason_for(context, pattern);

    Finding {
        id: String::new(),
        rule_id: RULE_ID.to_string(),
        recommendation: recommendation.to_string(),
        title: format!("Risky Rust {} usage in {}", pattern.label(), context_label),
        description: format!(
            "Rust `{}` was found in {}; confidence is {} because {}. Unhandled panic paths can terminate execution or hide recoverable errors in production code.",
            pattern.label(),
            context_label,
            confidence.label(),
            confidence_reason,
        ),
        category: FindingCategory::CodeQuality,
        severity,
        confidence,
        evidence: vec![Evidence {
            path: file.path.clone(),
            line_start: line_number,
            line_end: None,
            snippet: snippet.to_string(),
        }],
        workspace_package: None,
        docs_url: None,
        risk: Default::default(),
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

fn confidence_for(
    context: &crate::audits::context::AuditContext,
    pattern: RustPanicPattern,
    severity: Severity,
) -> Confidence {
    if context.is_test {
        return Confidence::Low;
    }

    if matches!(
        pattern,
        RustPanicPattern::Todo | RustPanicPattern::Unimplemented
    ) {
        return Confidence::High;
    }

    if context.has_runtime(RuntimeKind::RustCli) {
        return match pattern {
            RustPanicPattern::Unwrap | RustPanicPattern::Expect => Confidence::Low,
            RustPanicPattern::Panic => Confidence::Medium,
            RustPanicPattern::Todo | RustPanicPattern::Unimplemented => Confidence::High,
        };
    }

    if context.has_role(FileRole::Domain) || context.has_runtime(RuntimeKind::RustLibrary) {
        return Confidence::High;
    }

    match severity {
        Severity::High | Severity::Critical => Confidence::High,
        Severity::Info | Severity::Low => Confidence::Low,
        Severity::Medium => Confidence::Medium,
    }
}

fn confidence_reason_for(
    context: &crate::audits::context::AuditContext,
    pattern: RustPanicPattern,
) -> &'static str {
    if context.is_test {
        return "test code often uses panic-style macros for assertion setup or unfinished test scaffolding";
    }

    if matches!(
        pattern,
        RustPanicPattern::Todo | RustPanicPattern::Unimplemented
    ) {
        return "placeholder macros always panic if this path is reached";
    }

    if context.has_runtime(RuntimeKind::RustCli) {
        return match pattern {
            RustPanicPattern::Unwrap | RustPanicPattern::Expect => {
                "CLI boundary code may intentionally fail fast, but user-facing errors are usually better"
            }
            RustPanicPattern::Panic => {
                "CLI boundary code can terminate the process intentionally, but panic output is rarely a good user-facing error"
            }
            RustPanicPattern::Todo | RustPanicPattern::Unimplemented => {
                "placeholder macros always panic if this path is reached"
            }
        };
    }

    if context.has_role(FileRole::Domain) {
        return "domain code is usually reusable production logic, so callers cannot recover from this panic path";
    }

    if context.has_runtime(RuntimeKind::RustLibrary) {
        return "library code is reused by callers, so panics become part of its public failure behavior";
    }

    "this is production Rust code and the panic path is not locally handled"
}

fn recommendation_for(
    context: &crate::audits::context::AuditContext,
    pattern: RustPanicPattern,
) -> &'static str {
    if context.is_test {
        return "Panic-style helpers in tests can be acceptable, but keep them out of reusable test utilities when possible.";
    }

    match pattern {
        RustPanicPattern::Unwrap => {
            if context.has_runtime(RuntimeKind::RustCli) {
                "At CLI boundaries this may be acceptable for prototype code, but prefer returning a user-friendly error with context."
            } else {
                "Return `Result` or propagate with `?`; convert to `expect()` only when failure is impossible and the message documents the invariant."
            }
        }
        RustPanicPattern::Expect => {
            if context.has_runtime(RuntimeKind::RustCli) {
                "At CLI boundaries this may be acceptable for prototype code, but prefer returning a user-friendly error with context."
            } else {
                "Prefer `Result`/`?` for recoverable errors. Keep `expect()` only for impossible states, with a message that names the invariant."
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
    fn ignores_string_write_unwraps_in_report_renderers() {
        let file = facts(
            "src/output/markdown.rs",
            "pub fn render(output: &mut String) {\n    writeln!(output, \"# Report\").unwrap();\n}\n",
            false,
        );

        let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());

        assert!(findings.is_empty());
    }

    #[test]
    fn ignores_multiline_string_write_unwraps_in_report_renderers() {
        let file = facts(
            "src/output/console.rs",
            "pub fn render(output: &mut String) {\n    writeln!(\n        output,\n        \"Findings: {}\",\n        3\n    )\n    .unwrap();\n}\n",
            false,
        );

        let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());

        assert!(findings.is_empty());
    }

    #[test]
    fn still_reports_non_renderer_unwraps_in_output_modules() {
        let file = facts(
            "src/output/markdown.rs",
            "pub fn render(value: Option<&str>) -> &str {\n    value.unwrap()\n}\n",
            false,
        );

        let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, RULE_ID);
    }

    #[test]
    fn still_reports_unwraps_inside_renderer_format_arguments() {
        let file = facts(
            "src/output/console.rs",
            "pub fn render(output: &mut String, value: Option<&str>) {\n    writeln!(output, \"{}\", value.unwrap());\n}\n",
            false,
        );

        let findings = RustPanicRiskAudit.audit(&file, &ScanConfig::default());

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, RULE_ID);
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
            lines_of_code: content.lines().count(),
            branch_count: 0,
            imports: Vec::new(),
            content: Some(content.to_string()),
            has_inline_tests,
        }
    }
}
