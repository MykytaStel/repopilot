use crate::audits::context::{AuditContext, FileRole, FrameworkKind, LanguageKind, classify_file};
use crate::audits::traits::FileAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::scan::config::ScanConfig;
use crate::scan::facts::FileFacts;
use crate::scan::path_classification::is_low_signal_audit_path;
use std::path::Path;

mod brace;
mod python;

pub struct LongFunctionAudit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LongFunctionPolicy {
    pub threshold: usize,
    pub severity: Severity,
    pub context_label: &'static str,
    pub recommendation: &'static str,
}

impl FileAudit for LongFunctionAudit {
    fn audit(&self, file: &FileFacts, config: &ScanConfig) -> Vec<Finding> {
        if is_low_signal_audit_path(&file.path) {
            return vec![];
        }

        let content = file.content.as_deref().unwrap_or("");

        if content.is_empty() {
            return vec![];
        }

        let language = match file.language.as_deref() {
            Some(language) if is_supported(language) => language,
            _ => return vec![],
        };

        let context = classify_file(file);
        let policy = long_function_policy(&context, config.long_function_loc_threshold);

        detect_long_functions(content, language, &file.path, policy)
    }
}

fn is_supported(language: &str) -> bool {
    matches!(
        language,
        "Rust"
            | "Go"
            | "Python"
            | "TypeScript"
            | "TypeScript React"
            | "JavaScript"
            | "JavaScript React"
            | "Java"
            | "Kotlin"
            | "CSharp"
            | "C#"
            | "CS"
    )
}

fn detect_long_functions(
    content: &str,
    language: &str,
    path: &Path,
    policy: LongFunctionPolicy,
) -> Vec<Finding> {
    if language == "Python" {
        python::detect_python(content, path, policy)
    } else {
        brace::detect_brace_based(content, language, path, policy)
    }
}

fn long_function_policy(context: &AuditContext, base_threshold: usize) -> LongFunctionPolicy {
    if context.has_role(FileRole::Config) {
        return LongFunctionPolicy {
            threshold: usize::MAX,
            severity: Severity::Info,
            context_label: "configuration file",
            recommendation: "Configuration files are not evaluated with the generic long-function threshold.",
        };
    }

    if context.is_react_component() {
        return LongFunctionPolicy {
            threshold: base_threshold.saturating_mul(3),
            severity: Severity::Low,
            context_label: "React component",
            recommendation: "For large React components, prefer extracting child components, hooks, or view-model helpers instead of treating JSX layout as a regular long function.",
        };
    }

    if context.is_react_hook() {
        return LongFunctionPolicy {
            threshold: base_threshold.saturating_mul(2),
            severity: Severity::Low,
            context_label: "React hook",
            recommendation: "For large hooks, consider splitting state/effect orchestration from data mapping or side-effect helpers.",
        };
    }

    if context.has_role(FileRole::UnityMonoBehaviour) || context.has_framework(FrameworkKind::Unity)
    {
        return LongFunctionPolicy {
            threshold: base_threshold.saturating_mul(2),
            severity: Severity::Low,
            context_label: "Unity MonoBehaviour",
            recommendation: "For large Unity behaviours, consider moving domain logic out of lifecycle methods and into smaller services or components.",
        };
    }

    if context.has_role(FileRole::DotNetController) {
        return LongFunctionPolicy {
            threshold: base_threshold.saturating_mul(2),
            severity: Severity::Low,
            context_label: ".NET controller",
            recommendation: "For large controllers, prefer moving business logic into services, handlers, or application-layer use cases.",
        };
    }

    if context.has_role(FileRole::DotNetService) {
        return LongFunctionPolicy {
            threshold: base_threshold.saturating_mul(3) / 2,
            severity: Severity::Medium,
            context_label: ".NET service",
            recommendation: "For large services, consider splitting orchestration, validation, persistence, and external integration logic.",
        };
    }

    if context.has_role(FileRole::RustTest) || context.is_test {
        return LongFunctionPolicy {
            threshold: base_threshold.saturating_mul(2),
            severity: Severity::Low,
            context_label: "test code",
            recommendation: "For large tests, consider extracting builders, fixtures, or assertion helpers, but test setup can be naturally longer than production logic.",
        };
    }

    if context.language == LanguageKind::Rust {
        return LongFunctionPolicy {
            threshold: base_threshold,
            severity: Severity::Medium,
            context_label: "Rust production code",
            recommendation: "Consider extracting smaller functions or methods to isolate parsing, validation, IO, or transformation steps.",
        };
    }

    LongFunctionPolicy {
        threshold: base_threshold,
        severity: Severity::Medium,
        context_label: "generic production code",
        recommendation: "Long functions are harder to test and reason about — consider extracting helper functions.",
    }
}

fn build_finding(
    path: &Path,
    start_line: usize,
    end_line: usize,
    fn_name: &str,
    fn_len: usize,
    policy: LongFunctionPolicy,
) -> Finding {
    let (title, name_display) = if fn_name.is_empty() {
        (
            format!("Large {} inline function", policy.context_label),
            "inline function".to_string(),
        )
    } else {
        (
            format!("Large {} function: `{fn_name}`", policy.context_label),
            format!("`{fn_name}`"),
        )
    };

    Finding {
        id: String::new(),
        rule_id: "code-quality.long-function".to_string(),
        title,
        description: format!(
            "Function {name_display} spans {fn_len} lines, exceeding the context-aware {threshold}-line threshold for {context_label}. {recommendation}",
            threshold = policy.threshold,
            context_label = policy.context_label,
            recommendation = policy.recommendation,
        ),
        category: FindingCategory::CodeQuality,
        severity: policy.severity,
        evidence: vec![Evidence {
            path: path.to_path_buf(),
            line_start: start_line,
            line_end: Some(end_line),
            snippet: format!(
                "function spans lines {start_line}–{end_line} ({fn_len} lines, threshold {})",
                policy.threshold
            ),
        }],
        workspace_package: None,
        docs_url: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::facts::FileFacts;
    use std::path::PathBuf;

    #[test]
    fn uses_larger_threshold_for_react_component() {
        let file = facts(
            "src/components/ProfileCard.tsx",
            Some("TypeScript React"),
            &large_react_component(80),
            false,
        );

        let findings = LongFunctionAudit.audit(
            &file,
            &ScanConfig {
                long_function_loc_threshold: 50,
                ..ScanConfig::default()
            },
        );

        assert!(
            findings.is_empty(),
            "React component should not be flagged by the generic 50-line threshold"
        );
    }

    #[test]
    fn still_reports_very_large_react_component_with_low_severity() {
        let file = facts(
            "src/components/ProfileCard.tsx",
            Some("TypeScript React"),
            &large_react_component(170),
            false,
        );

        let findings = LongFunctionAudit.audit(
            &file,
            &ScanConfig {
                long_function_loc_threshold: 50,
                ..ScanConfig::default()
            },
        );

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Low);
        assert!(findings[0].title.contains("React component"));
    }

    #[test]
    fn uses_larger_threshold_for_react_hook() {
        let file = facts(
            "src/hooks/useProfile.ts",
            Some("TypeScript"),
            &large_hook(80),
            false,
        );

        let findings = LongFunctionAudit.audit(
            &file,
            &ScanConfig {
                long_function_loc_threshold: 50,
                ..ScanConfig::default()
            },
        );

        assert!(findings.is_empty());
    }

    #[test]
    fn still_reports_generic_typescript_utility() {
        let file = facts(
            "src/utils/buildPayload.ts",
            Some("TypeScript"),
            &large_typescript_function(60),
            false,
        );

        let findings = LongFunctionAudit.audit(
            &file,
            &ScanConfig {
                long_function_loc_threshold: 50,
                ..ScanConfig::default()
            },
        );

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Medium);
        assert!(findings[0].title.contains("generic production code"));
    }

    #[test]
    fn uses_larger_threshold_for_unity_monobehaviour() {
        let file = facts(
            "Assets/Scripts/PlayerController.cs",
            Some("CSharp"),
            &large_unity_method(80),
            false,
        );

        let findings = LongFunctionAudit.audit(
            &file,
            &ScanConfig {
                long_function_loc_threshold: 50,
                ..ScanConfig::default()
            },
        );

        assert!(findings.is_empty());
    }

    #[test]
    fn uses_larger_threshold_for_dotnet_controller() {
        let file = facts(
            "src/Controllers/UsersController.cs",
            Some("CSharp"),
            &large_dotnet_controller_action(80),
            false,
        );

        let findings = LongFunctionAudit.audit(
            &file,
            &ScanConfig {
                long_function_loc_threshold: 50,
                ..ScanConfig::default()
            },
        );

        assert!(findings.is_empty());
    }

    #[test]
    fn uses_larger_threshold_for_rust_tests() {
        let file = facts(
            "tests/integration_test.rs",
            Some("Rust"),
            &large_rust_test(80),
            false,
        );

        let findings = LongFunctionAudit.audit(
            &file,
            &ScanConfig {
                long_function_loc_threshold: 50,
                ..ScanConfig::default()
            },
        );

        assert!(findings.is_empty());
    }

    fn facts(
        path: &str,
        language: Option<&str>,
        content: &str,
        has_inline_tests: bool,
    ) -> FileFacts {
        FileFacts {
            path: PathBuf::from(path),
            language: language.map(str::to_string),
            lines_of_code: content.lines().count(),
            branch_count: 0,
            imports: Vec::new(),
            content: Some(content.to_string()),
            has_inline_tests,
        }
    }

    fn large_react_component(body_lines: usize) -> String {
        let body = repeated_lines(body_lines, "  <Text>Profile</Text>");

        format!(
            "import React from 'react';\nexport function ProfileCard() {{\n  return (\n    <View>\n{body}\n    </View>\n  );\n}}\n"
        )
    }

    fn large_hook(body_lines: usize) -> String {
        let body = repeated_lines(body_lines, "  const value = state + 1;");

        format!(
            "import {{ useEffect, useState }} from 'react';\nexport function useProfile() {{\n  const [state, setState] = useState(0);\n  useEffect(() => {{ setState(1); }}, []);\n{body}\n  return state;\n}}\n"
        )
    }

    fn large_typescript_function(body_lines: usize) -> String {
        let body = repeated_lines(body_lines, "  payload.items.push(item);");

        format!("export function buildPayload(payload: any) {{\n{body}\n  return payload;\n}}\n")
    }

    fn large_unity_method(body_lines: usize) -> String {
        let body = repeated_lines(body_lines, "        transform.position += Vector3.forward;");

        format!(
            "using UnityEngine;\npublic class PlayerController : MonoBehaviour {{\n    void Update() {{\n{body}\n    }}\n}}\n"
        )
    }

    fn large_dotnet_controller_action(body_lines: usize) -> String {
        let body = repeated_lines(body_lines, "        var value = request.Id;");

        format!(
            "using Microsoft.AspNetCore.Mvc;\n[ApiController]\npublic class UsersController : ControllerBase {{\n    public IActionResult GetUser(Request request) {{\n{body}\n        return Ok();\n    }}\n}}\n"
        )
    }

    fn large_rust_test(body_lines: usize) -> String {
        let body = repeated_lines(body_lines, "    let value = 1 + 1;");

        format!("#[test]\nfn long_integration_test() {{\n{body}\n    assert_eq!(value, 2);\n}}\n")
    }

    fn repeated_lines(count: usize, line: &str) -> String {
        std::iter::repeat_n(line, count)
            .collect::<Vec<_>>()
            .join("\n")
    }
}
