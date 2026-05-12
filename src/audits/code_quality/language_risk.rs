use crate::audits::traits::FileAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::knowledge::decision::decide_for_file;
use crate::knowledge::language::language_id_for_name;
use crate::scan::config::ScanConfig;
use crate::scan::facts::FileFacts;
use std::path::Path;

pub struct LanguageRiskAudit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LanguageRiskPattern {
    rule_id: &'static str,
    signal: &'static str,
    title: &'static str,
    context_label: &'static str,
    recommendation: &'static str,
    base_severity: Severity,
}

impl FileAudit for LanguageRiskAudit {
    fn audit(&self, file: &FileFacts, _config: &ScanConfig) -> Vec<Finding> {
        let Some(content) = file
            .content
            .as_deref()
            .filter(|content| !content.is_empty())
        else {
            return vec![];
        };

        let Some(language_id) = file.language.as_deref().and_then(language_id_for_name) else {
            return vec![];
        };

        detect_language_risks(language_id, content, &file.path, file)
    }
}

fn detect_language_risks(
    language_id: &str,
    content: &str,
    path: &Path,
    file: &FileFacts,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut in_block_comment = false;

    for (line_index, raw_line) in content.lines().enumerate() {
        let Some(line) = sanitized_code_line(raw_line, &mut in_block_comment) else {
            continue;
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let patterns = patterns_for_line(language_id, trimmed, path);
        for pattern in patterns {
            let decision = decide_for_file(
                pattern.rule_id,
                file,
                pattern.base_severity,
                Some(pattern.signal),
            );
            if decision.is_suppressed() {
                continue;
            }

            findings.push(build_finding(
                path,
                line_index + 1,
                raw_line.trim(),
                pattern,
                decision.severity,
            ));
        }
    }

    findings
}

fn patterns_for_line(language_id: &str, trimmed: &str, path: &Path) -> Vec<LanguageRiskPattern> {
    match language_id {
        "go" => go_patterns(trimmed),
        "python" => python_patterns(trimmed),
        "typescript" | "typescript-react" | "javascript" | "javascript-react" => {
            js_patterns(trimmed, path)
        }
        "java" | "kotlin" | "csharp" => managed_patterns(language_id, trimmed),
        _ => Vec::new(),
    }
}

fn go_patterns(trimmed: &str) -> Vec<LanguageRiskPattern> {
    let mut patterns = Vec::new();
    if trimmed.contains("panic(") {
        patterns.push(pattern(
            "language.go.panic-exit-risk",
            "go.panic",
            "Risky Go panic usage",
            "Go panic call",
            "Return an error from reusable Go code and let the caller decide how to recover.",
            Severity::Medium,
        ));
    }
    if trimmed.contains("log.Fatal(") || trimmed.contains("log.Fatalf(") {
        patterns.push(pattern(
            "language.go.panic-exit-risk",
            "go.log-fatal",
            "Risky Go log.Fatal usage",
            "Go fatal logging call",
            "Use returned errors outside the narrow CLI boundary so libraries remain recoverable.",
            Severity::Medium,
        ));
    }
    if trimmed.contains("os.Exit(") {
        patterns.push(pattern(
            "language.go.panic-exit-risk",
            "go.os-exit",
            "Risky Go os.Exit usage",
            "Go process exit call",
            "Centralise process exits at the CLI entrypoint and return errors elsewhere.",
            Severity::Medium,
        ));
    }
    patterns
}

fn python_patterns(trimmed: &str) -> Vec<LanguageRiskPattern> {
    let mut patterns = Vec::new();
    if is_broad_python_except(trimmed) {
        patterns.push(pattern(
            "language.python.exception-risk",
            "python.broad-except",
            "Broad Python except handler",
            "Python broad exception handler",
            "Catch specific exceptions so unrelated failures are not hidden.",
            Severity::Medium,
        ));
    }
    if trimmed.starts_with("assert ") || trimmed.starts_with("assert(") {
        patterns.push(pattern(
            "language.python.exception-risk",
            "python.assert",
            "Python assert in production path",
            "Python assert statement",
            "Use explicit runtime validation for production invariants because asserts can be disabled.",
            Severity::Medium,
        ));
    }
    if trimmed.contains("raise NotImplementedError")
        || trimmed.contains("NotImplementedError(")
        || trimmed == "raise NotImplementedError"
    {
        patterns.push(pattern(
            "language.python.exception-risk",
            "python.not-implemented",
            "Python NotImplementedError placeholder",
            "Python not-implemented placeholder",
            "Replace placeholders before production release or guard them behind explicit feature flags.",
            Severity::High,
        ));
    }
    patterns
}

fn js_patterns(trimmed: &str, path: &Path) -> Vec<LanguageRiskPattern> {
    let mut patterns = Vec::new();
    if trimmed.contains("process.exit(") {
        patterns.push(pattern(
            "language.javascript.runtime-exit-risk",
            "js.process-exit",
            "JavaScript process.exit usage",
            "JavaScript process exit call",
            "Keep process exits at a CLI boundary and return errors from reusable modules.",
            Severity::Medium,
        ));
    }
    if trimmed.contains("throw new Error(") && is_library_boundary_path(path) {
        patterns.push(pattern(
            "language.javascript.runtime-exit-risk",
            "js.throw-error",
            "Generic JavaScript error at library boundary",
            "JavaScript generic thrown error",
            "Prefer typed errors or actionable error messages at reusable package boundaries.",
            Severity::Medium,
        ));
    }
    patterns
}

fn managed_patterns(language_id: &str, trimmed: &str) -> Vec<LanguageRiskPattern> {
    let mut patterns = Vec::new();
    let is_csharp = language_id == "csharp";

    if trimmed.contains("throw new RuntimeException(")
        || trimmed.contains("throw new IllegalStateException(")
        || (is_csharp && trimmed.contains("throw new Exception("))
    {
        patterns.push(pattern(
            "language.managed.fatal-exception-risk",
            "managed.fatal-exception",
            "Generic fatal exception in managed code",
            "JVM/.NET generic fatal exception",
            "Use domain-specific exception or result types when callers need precise recovery behaviour.",
            Severity::Medium,
        ));
    }

    if trimmed.contains("throw new NotImplementedException(")
        || trimmed.contains("throw new NotImplementedError(")
        || trimmed.contains("TODO(")
        || trimmed.contains("TODO()")
    {
        patterns.push(pattern(
            "language.managed.fatal-exception-risk",
            "managed.not-implemented",
            "Not-implemented placeholder in managed code",
            "JVM/.NET placeholder failure",
            "Replace placeholders before production release or isolate unfinished paths clearly.",
            Severity::High,
        ));
    }

    patterns
}

fn pattern(
    rule_id: &'static str,
    signal: &'static str,
    title: &'static str,
    context_label: &'static str,
    recommendation: &'static str,
    base_severity: Severity,
) -> LanguageRiskPattern {
    LanguageRiskPattern {
        rule_id,
        signal,
        title,
        context_label,
        recommendation,
        base_severity,
    }
}

fn build_finding(
    path: &Path,
    line_number: usize,
    snippet: &str,
    pattern: LanguageRiskPattern,
    severity: Severity,
) -> Finding {
    Finding {
        id: String::new(),
        rule_id: pattern.rule_id.to_string(),
        title: pattern.title.to_string(),
        description: format!(
            "{} was found in {}. {}",
            pattern.context_label,
            path.display(),
            pattern.recommendation
        ),
        category: FindingCategory::CodeQuality,
        severity,
        evidence: vec![Evidence {
            path: path.to_path_buf(),
            line_start: line_number,
            line_end: None,
            snippet: snippet.to_string(),
        }],
        workspace_package: None,
        docs_url: None,
    }
}

fn sanitized_code_line(line: &str, in_block_comment: &mut bool) -> Option<String> {
    let trimmed = line.trim();
    if *in_block_comment {
        if trimmed.contains("*/") {
            *in_block_comment = false;
        }
        return None;
    }
    if trimmed.starts_with("//")
        || trimmed.starts_with('#')
        || trimmed.starts_with("///")
        || trimmed.starts_with("/*")
        || trimmed.starts_with('*')
    {
        if trimmed.starts_with("/*") && !trimmed.contains("*/") {
            *in_block_comment = true;
        }
        return None;
    }

    Some(strip_string_literals(line))
}

fn strip_string_literals(line: &str) -> String {
    let mut result = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    let mut string_delimiter = None;
    let mut escaped = false;

    while let Some(character) = chars.next() {
        if let Some(delimiter) = string_delimiter {
            if escaped {
                escaped = false;
                result.push(' ');
                continue;
            }
            if character == '\\' {
                escaped = true;
                result.push(' ');
                continue;
            }
            if character == delimiter {
                string_delimiter = None;
                result.push(character);
            } else {
                result.push(' ');
            }
            continue;
        }

        if character == '/' && chars.peek() == Some(&'/') {
            break;
        }
        if character == '#' {
            break;
        }
        if matches!(character, '"' | '\'' | '`') {
            string_delimiter = Some(character);
            result.push(character);
        } else {
            result.push(character);
        }
    }

    result
}

fn is_broad_python_except(trimmed: &str) -> bool {
    let normalized = trimmed.replace(' ', "");
    normalized == "except:" || normalized.starts_with("except:#")
}

fn is_library_boundary_path(path: &Path) -> bool {
    path.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .map(|value| {
                matches!(
                    value.to_lowercase().as_str(),
                    "domain"
                        | "domains"
                        | "core"
                        | "model"
                        | "models"
                        | "lib"
                        | "libs"
                        | "packages"
                )
            })
            .unwrap_or(false)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn reports_go_panic_in_domain_code_as_high() {
        let file = facts(
            "src/domain/user.go",
            Some("Go"),
            "package domain\nfunc Parse() { panic(\"bad\") }\n",
        );

        let findings = LanguageRiskAudit.audit(&file, &ScanConfig::default());

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "language.go.panic-exit-risk");
        assert_eq!(findings[0].severity, Severity::High);
    }

    #[test]
    fn downgrades_python_assert_in_tests() {
        let file = facts(
            "tests/test_user.py",
            Some("Python"),
            "assert user.is_valid\n",
        );

        let findings = LanguageRiskAudit.audit(&file, &ScanConfig::default());

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Low);
    }

    #[test]
    fn reports_js_process_exit_but_not_string_literal() {
        let file = facts(
            "src/cli/main.ts",
            Some("TypeScript"),
            "const text = \"process.exit(1)\";\nprocess.exit(1);\n",
        );

        let findings = LanguageRiskAudit.audit(&file, &ScanConfig::default());

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "language.javascript.runtime-exit-risk");
    }

    #[test]
    fn reports_csharp_not_implemented_placeholder() {
        let file = facts(
            "src/domain/UserService.cs",
            Some("C#"),
            "public void Run() { throw new NotImplementedException(); }\n",
        );

        let findings = LanguageRiskAudit.audit(&file, &ScanConfig::default());

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "language.managed.fatal-exception-risk");
        assert_eq!(findings[0].severity, Severity::High);
    }

    #[test]
    fn ignores_functional_iterator_style_without_risky_pattern() {
        let file = facts(
            "src/domain/users.ts",
            Some("TypeScript"),
            "export const names = users.filter(u => u.active).map(u => u.name);\n",
        );

        let findings = LanguageRiskAudit.audit(&file, &ScanConfig::default());

        assert!(findings.is_empty());
    }

    fn facts(path: &str, language: Option<&str>, content: &str) -> FileFacts {
        FileFacts {
            path: PathBuf::from(path),
            language: language.map(str::to_string),
            lines_of_code: content.lines().count(),
            branch_count: 0,
            imports: Vec::new(),
            content: Some(content.to_string()),
            has_inline_tests: false,
        }
    }
}
