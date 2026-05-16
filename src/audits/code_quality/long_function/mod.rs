use crate::audits::context::{AuditContext, FileRole, FrameworkKind, LanguageKind, classify_file};
use crate::audits::traits::FileAudit;
use crate::findings::types::{Confidence, Evidence, Finding, FindingCategory, Severity};
use crate::knowledge::decision::decide_for_audit_context;
use crate::scan::config::ScanConfig;
use crate::scan::facts::FileFacts;
use crate::scan::path_classification::is_low_signal_audit_path;
use std::path::Path;

mod brace;
mod python;

const RULE_ID: &str = "code-quality.long-function";

pub struct LongFunctionAudit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LongFunctionPolicy {
    pub threshold: usize,
    pub severity: Severity,
    pub confidence: Confidence,
    pub context_label: &'static str,
    pub confidence_reason: &'static str,
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

        let Some(language) = file.language.as_deref() else {
            return vec![];
        };

        let context = classify_file(file);
        let decision = decide_for_audit_context(RULE_ID, &context, Severity::Medium, None);

        if decision.is_suppressed() {
            return vec![];
        }

        let mut policy = long_function_policy(&context, config.long_function_loc_threshold);
        policy.severity = decision.severity.min(policy.severity);

        detect_long_functions(content, language, &file.path, policy)
    }
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
            confidence: Confidence::Low,
            context_label: "configuration file",
            confidence_reason: "configuration files often encode declarative setup rather than executable business logic",
            recommendation: "Configuration files are not evaluated with the generic long-function threshold.",
        };
    }

    if context.is_react_component() {
        return LongFunctionPolicy {
            threshold: base_threshold.saturating_mul(3),
            severity: Severity::Low,
            confidence: Confidence::Low,
            context_label: "React component",
            confidence_reason: "JSX, hooks, and layout markup often make component files longer without implying mixed responsibilities",
            recommendation: "Split only if state, effects, rendering, or data-shaping concerns are mixed. Prefer extracting child components, hooks, or view-model helpers at clear boundaries.",
        };
    }

    if context.is_react_hook() {
        return LongFunctionPolicy {
            threshold: base_threshold.saturating_mul(2),
            severity: Severity::Low,
            confidence: Confidence::Low,
            context_label: "React hook",
            confidence_reason: "hooks often combine state and effect orchestration that can be longer than a pure helper function",
            recommendation: "For large hooks, consider splitting state/effect orchestration from data mapping or side-effect helpers.",
        };
    }

    if context.has_role(FileRole::UnityMonoBehaviour) || context.has_framework(FrameworkKind::Unity)
    {
        return LongFunctionPolicy {
            threshold: base_threshold.saturating_mul(2),
            severity: Severity::Low,
            confidence: Confidence::Low,
            context_label: "Unity MonoBehaviour",
            confidence_reason: "engine lifecycle methods can accumulate setup and event wiring that is not always business logic",
            recommendation: "For large Unity behaviours, consider moving domain logic out of lifecycle methods and into smaller services or components.",
        };
    }

    if context.has_role(FileRole::DotNetController) {
        return LongFunctionPolicy {
            threshold: base_threshold.saturating_mul(2),
            severity: Severity::Low,
            confidence: Confidence::Low,
            context_label: ".NET controller",
            confidence_reason: "controller actions often include request/response wiring that can be longer than domain logic",
            recommendation: "For large controllers, prefer moving business logic into services, handlers, or application-layer use cases.",
        };
    }

    if context.has_role(FileRole::DotNetService) {
        return LongFunctionPolicy {
            threshold: base_threshold.saturating_mul(3) / 2,
            severity: Severity::Medium,
            confidence: Confidence::High,
            context_label: ".NET service",
            confidence_reason: "service classes usually carry reusable production orchestration where long methods signal mixed responsibilities",
            recommendation: "For large services, consider splitting orchestration, validation, persistence, and external integration logic.",
        };
    }

    if context.has_role(FileRole::RustTest) || context.is_test {
        return LongFunctionPolicy {
            threshold: base_threshold.saturating_mul(2),
            severity: Severity::Low,
            confidence: Confidence::Low,
            context_label: "test code",
            confidence_reason: "tests often include setup and assertions that are naturally longer than production helpers",
            recommendation: "For large tests, consider extracting builders, fixtures, or assertion helpers, but test setup can be naturally longer than production logic.",
        };
    }

    if context.language == LanguageKind::Rust {
        return LongFunctionPolicy {
            threshold: base_threshold,
            severity: Severity::Medium,
            confidence: Confidence::High,
            context_label: "Rust production code",
            confidence_reason: "production Rust functions are usually explicit control-flow units where length increases review and error-handling risk",
            recommendation: "Consider extracting smaller functions or methods to isolate parsing, validation, IO, or transformation steps.",
        };
    }

    LongFunctionPolicy {
        threshold: base_threshold,
        severity: Severity::Medium,
        confidence: Confidence::High,
        context_label: "generic production code",
        confidence_reason: "production functions that exceed the threshold are more likely to mix responsibilities",
        recommendation: "Long functions are harder to test and reason about; consider extracting helper functions.",
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
        rule_id: RULE_ID.to_string(),
        recommendation: policy.recommendation.to_string(),
        title,
        description: format!(
            "This is a long function in {context_label}; confidence is {confidence} because {confidence_reason}. Function {name_display} spans {fn_len} lines, exceeding the context-aware {threshold}-line threshold. Large functions are harder to review, test, and safely change.",
            confidence = policy.confidence.label(),
            confidence_reason = policy.confidence_reason,
            threshold = policy.threshold,
            context_label = policy.context_label,
        ),
        category: FindingCategory::CodeQuality,
        severity: policy.severity,
        confidence: policy.confidence,
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
        risk: Default::default(),
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
        assert_eq!(findings[0].confidence, Confidence::Low);
        assert!(findings[0].title.contains("React component"));
        assert!(
            findings[0]
                .description
                .contains("confidence is LOW because JSX")
        );
        assert!(findings[0].recommendation.contains("Split only if state"));
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
        assert_eq!(findings[0].confidence, Confidence::High);
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
