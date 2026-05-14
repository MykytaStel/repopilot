use crate::audits::code_quality::sanitize::{sanitize_c_style, sanitize_python_line};
use crate::audits::traits::FileAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::knowledge::decision::decide_for_file;
use crate::knowledge::language::language_id_for_name;
use crate::scan::config::ScanConfig;
use crate::scan::facts::FileFacts;
use std::path::Path;

pub struct LanguageRiskAudit;

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
        let sanitized = if language_id == "python" {
            match sanitize_python_line(raw_line) {
                Some(s) => s,
                None => continue,
            }
        } else {
            sanitize_c_style(raw_line, &mut in_block_comment)
        };

        let trimmed = sanitized.trim();
        if trimmed.is_empty() {
            continue;
        }

        emit_findings_for_line(
            language_id,
            trimmed,
            path,
            raw_line,
            line_index,
            file,
            &mut findings,
        );
    }

    findings
}

fn emit_findings_for_line(
    language_id: &str,
    trimmed: &str,
    path: &Path,
    raw_line: &str,
    line_index: usize,
    file: &FileFacts,
    findings: &mut Vec<Finding>,
) {
    macro_rules! push_if_matched {
        ($pattern:expr) => {
            if $pattern.matches(trimmed, path) {
                let decision = decide_for_file(
                    $pattern.rule_id(),
                    file,
                    $pattern.base_severity(),
                    Some($pattern.signal()),
                );
                if !decision.is_suppressed() {
                    findings.push(build_finding(
                        path,
                        line_index + 1,
                        raw_line.trim(),
                        $pattern,
                        decision.severity,
                    ));
                }
            }
        };
    }

    match language_id {
        "go" => {
            for p in GoRiskPattern::ALL {
                push_if_matched!(p);
            }
        }
        "python" => {
            for p in PythonRiskPattern::ALL {
                push_if_matched!(p);
            }
        }
        "typescript" | "typescript-react" | "javascript" | "javascript-react" => {
            for p in JsRiskPattern::ALL {
                push_if_matched!(p);
            }
        }
        "java" | "kotlin" => {
            for p in ManagedRiskPattern::JVM_PATTERNS {
                push_if_matched!(p);
            }
        }
        "csharp" => {
            for p in ManagedRiskPattern::CSHARP_PATTERNS {
                push_if_matched!(p);
            }
        }
        _ => {}
    }
}

// ── Go ────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
enum GoRiskPattern {
    Panic,
    LogFatal,
    OsExit,
}

impl GoRiskPattern {
    const ALL: &'static [Self] = &[Self::Panic, Self::LogFatal, Self::OsExit];

    fn matches(self, trimmed: &str, _path: &Path) -> bool {
        match self {
            Self::Panic => trimmed.contains("panic("),
            Self::LogFatal => trimmed.contains("log.Fatal(") || trimmed.contains("log.Fatalf("),
            Self::OsExit => trimmed.contains("os.Exit("),
        }
    }

    fn rule_id(self) -> &'static str {
        "language.go.panic-exit-risk"
    }

    fn signal(self) -> &'static str {
        match self {
            Self::Panic => "go.panic",
            Self::LogFatal => "go.log-fatal",
            Self::OsExit => "go.os-exit",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::Panic => "Risky Go panic usage",
            Self::LogFatal => "Risky Go log.Fatal usage",
            Self::OsExit => "Risky Go os.Exit usage",
        }
    }

    fn context_label(self) -> &'static str {
        match self {
            Self::Panic => "Go panic call",
            Self::LogFatal => "Go fatal logging call",
            Self::OsExit => "Go process exit call",
        }
    }

    fn recommendation(self) -> &'static str {
        match self {
            Self::Panic => {
                "Return an error from reusable Go code and let the caller decide how to recover."
            }
            Self::LogFatal => {
                "Use returned errors outside the narrow CLI boundary so libraries remain recoverable."
            }
            Self::OsExit => {
                "Centralise process exits at the CLI entrypoint and return errors elsewhere."
            }
        }
    }

    fn base_severity(self) -> Severity {
        Severity::Medium
    }
}

// ── Python ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
enum PythonRiskPattern {
    BroadExcept,
    Assert,
    NotImplemented,
}

impl PythonRiskPattern {
    const ALL: &'static [Self] = &[Self::BroadExcept, Self::Assert, Self::NotImplemented];

    fn matches(self, trimmed: &str, _path: &Path) -> bool {
        match self {
            Self::BroadExcept => {
                let normalized = trimmed.replace(' ', "");
                normalized == "except:" || normalized.starts_with("except:#")
            }
            Self::Assert => trimmed.starts_with("assert ") || trimmed.starts_with("assert("),
            Self::NotImplemented => {
                trimmed.contains("raise NotImplementedError")
                    || trimmed.contains("NotImplementedError(")
                    || trimmed == "raise NotImplementedError"
            }
        }
    }

    fn rule_id(self) -> &'static str {
        "language.python.exception-risk"
    }

    fn signal(self) -> &'static str {
        match self {
            Self::BroadExcept => "python.broad-except",
            Self::Assert => "python.assert",
            Self::NotImplemented => "python.not-implemented",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::BroadExcept => "Broad Python except handler",
            Self::Assert => "Python assert in production path",
            Self::NotImplemented => "Python NotImplementedError placeholder",
        }
    }

    fn context_label(self) -> &'static str {
        match self {
            Self::BroadExcept => "Python broad exception handler",
            Self::Assert => "Python assert statement",
            Self::NotImplemented => "Python not-implemented placeholder",
        }
    }

    fn recommendation(self) -> &'static str {
        match self {
            Self::BroadExcept => "Catch specific exceptions so unrelated failures are not hidden.",
            Self::Assert => {
                "Use explicit runtime validation for production invariants because asserts can be disabled."
            }
            Self::NotImplemented => {
                "Replace placeholders before production release or guard them behind explicit feature flags."
            }
        }
    }

    fn base_severity(self) -> Severity {
        match self {
            Self::NotImplemented => Severity::High,
            _ => Severity::Medium,
        }
    }
}

// ── JavaScript / TypeScript ───────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
enum JsRiskPattern {
    ProcessExit,
    ThrowError,
}

impl JsRiskPattern {
    const ALL: &'static [Self] = &[Self::ProcessExit, Self::ThrowError];

    fn matches(self, trimmed: &str, path: &Path) -> bool {
        match self {
            Self::ProcessExit => trimmed.contains("process.exit("),
            Self::ThrowError => {
                trimmed.contains("throw new Error(") && is_library_boundary_path(path)
            }
        }
    }

    fn rule_id(self) -> &'static str {
        "language.javascript.runtime-exit-risk"
    }

    fn signal(self) -> &'static str {
        match self {
            Self::ProcessExit => "js.process-exit",
            Self::ThrowError => "js.throw-error",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::ProcessExit => "JavaScript process.exit usage",
            Self::ThrowError => "Generic JavaScript error at library boundary",
        }
    }

    fn context_label(self) -> &'static str {
        match self {
            Self::ProcessExit => "JavaScript process exit call",
            Self::ThrowError => "JavaScript generic thrown error",
        }
    }

    fn recommendation(self) -> &'static str {
        match self {
            Self::ProcessExit => {
                "Keep process exits at a CLI boundary and return errors from reusable modules."
            }
            Self::ThrowError => {
                "Prefer typed errors or actionable error messages at reusable package boundaries."
            }
        }
    }

    fn base_severity(self) -> Severity {
        Severity::Medium
    }
}

// ── Java / Kotlin / C# ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
enum ManagedRiskPattern {
    FatalException { is_csharp: bool },
    NotImplemented,
}

impl ManagedRiskPattern {
    const JVM_PATTERNS: &'static [Self] = &[
        Self::FatalException { is_csharp: false },
        Self::NotImplemented,
    ];
    const CSHARP_PATTERNS: &'static [Self] = &[
        Self::FatalException { is_csharp: true },
        Self::NotImplemented,
    ];

    fn matches(self, trimmed: &str, _path: &Path) -> bool {
        match self {
            Self::FatalException { is_csharp } => {
                trimmed.contains("throw new RuntimeException(")
                    || trimmed.contains("throw new IllegalStateException(")
                    || (is_csharp && trimmed.contains("throw new Exception("))
            }
            Self::NotImplemented => {
                trimmed.contains("throw new NotImplementedException(")
                    || trimmed.contains("throw new NotImplementedError(")
                    || trimmed.contains("TODO(")
                    || trimmed.contains("TODO()")
            }
        }
    }

    fn rule_id(self) -> &'static str {
        "language.managed.fatal-exception-risk"
    }

    fn signal(self) -> &'static str {
        match self {
            Self::FatalException { .. } => "managed.fatal-exception",
            Self::NotImplemented => "managed.not-implemented",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::FatalException { .. } => "Generic fatal exception in managed code",
            Self::NotImplemented => "Not-implemented placeholder in managed code",
        }
    }

    fn context_label(self) -> &'static str {
        match self {
            Self::FatalException { .. } => "JVM/.NET generic fatal exception",
            Self::NotImplemented => "JVM/.NET placeholder failure",
        }
    }

    fn recommendation(self) -> &'static str {
        match self {
            Self::FatalException { .. } => {
                "Use domain-specific exception or result types when callers need precise recovery behaviour."
            }
            Self::NotImplemented => {
                "Replace placeholders before production release or isolate unfinished paths clearly."
            }
        }
    }

    fn base_severity(self) -> Severity {
        match self {
            Self::NotImplemented => Severity::High,
            _ => Severity::Medium,
        }
    }
}

// ── Shared ────────────────────────────────────────────────────────────────────

trait RiskPattern {
    fn matches(&self, trimmed: &str, path: &Path) -> bool;
    fn rule_id(&self) -> &'static str;
    fn signal(&self) -> &'static str;
    fn title(&self) -> &'static str;
    fn context_label(&self) -> &'static str;
    fn recommendation(&self) -> &'static str;
    fn base_severity(&self) -> Severity;
}

macro_rules! impl_risk_pattern {
    ($t:ty) => {
        impl RiskPattern for $t {
            fn matches(&self, trimmed: &str, path: &Path) -> bool {
                (*self).matches(trimmed, path)
            }
            fn rule_id(&self) -> &'static str {
                (*self).rule_id()
            }
            fn signal(&self) -> &'static str {
                (*self).signal()
            }
            fn title(&self) -> &'static str {
                (*self).title()
            }
            fn context_label(&self) -> &'static str {
                (*self).context_label()
            }
            fn recommendation(&self) -> &'static str {
                (*self).recommendation()
            }
            fn base_severity(&self) -> Severity {
                (*self).base_severity()
            }
        }
    };
}

impl_risk_pattern!(GoRiskPattern);
impl_risk_pattern!(PythonRiskPattern);
impl_risk_pattern!(JsRiskPattern);
impl_risk_pattern!(ManagedRiskPattern);

fn build_finding(
    path: &Path,
    line_number: usize,
    snippet: &str,
    pattern: &dyn RiskPattern,
    severity: Severity,
) -> Finding {
    Finding {
        id: String::new(),
        rule_id: pattern.rule_id().to_string(),
        recommendation: pattern.recommendation().to_string(),
        title: pattern.title().to_string(),
        description: format!(
            "{} was found in {}. Runtime termination and placeholder exception paths can bypass normal error handling and make failures harder to recover from.",
            pattern.context_label(),
            path.display(),
        ),
        category: FindingCategory::CodeQuality,
        severity,
        confidence: Default::default(),
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
