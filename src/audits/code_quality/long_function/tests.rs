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

fn facts(path: &str, language: Option<&str>, content: &str, has_inline_tests: bool) -> FileFacts {
    FileFacts {
        path: PathBuf::from(path),
        language: language.map(str::to_string),
        non_empty_lines: content.lines().count(),
        branch_count: 0,
        imports: Vec::new(),
        content: Some(content.to_string()),
        has_inline_tests,
        in_executable_package: false,
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
