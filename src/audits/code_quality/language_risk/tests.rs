use super::*;
use crate::findings::types::Severity;
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
fn downgrades_js_process_exit_in_script_paths() {
    let file = facts("scripts/check.js", Some("JavaScript"), "process.exit(1);\n");

    let findings = LanguageRiskAudit.audit(&file, &ScanConfig::default());

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Low);
}

#[test]
fn reports_js_process_exit_in_library_code_as_high() {
    let file = facts(
        "src/lib/runtime.js",
        Some("JavaScript"),
        "export function stop() { process.exit(1); }\n",
    );

    let findings = LanguageRiskAudit.audit(&file, &ScanConfig::default());

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::High);
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

// ── AST detection: provenance + structural precision ────────────────────────────

#[test]
fn ast_runtime_finding_reports_ast_provenance() {
    use crate::rules::SignalSource;

    let file = facts(
        "src/lib/runtime.js",
        Some("JavaScript"),
        "export function stop() { process.exit(1); }\n",
    );

    let mut findings = LanguageRiskAudit.audit(&file, &ScanConfig::default());
    assert_eq!(findings.len(), 1);

    // Enrichment fills provenance from rule metadata for default-provenance
    // findings, so an AST-path finding must resolve to `ast`, not text-heuristic.
    findings[0].populate_rule_metadata();
    assert_eq!(findings[0].provenance.signal_source, SignalSource::Ast);
}

#[test]
fn python_bare_raise_not_implemented_is_flagged_exactly_once() {
    let file = facts(
        "src/service.py",
        Some("Python"),
        "def handler():\n    raise NotImplementedError\n",
    );

    let findings = LanguageRiskAudit.audit(&file, &ScanConfig::default());

    // The bare-identifier raise and the call form must not both match.
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "language.python.exception-risk");
}

#[test]
fn js_throw_new_error_outside_library_boundary_is_not_flagged() {
    let file = facts(
        "src/cli/main.ts",
        Some("TypeScript"),
        "export function run() { throw new Error(\"boom\"); }\n",
    );

    let findings = LanguageRiskAudit.audit(&file, &ScanConfig::default());

    // `throw new Error` is only risky at a reusable library boundary, not in CLI
    // entrypoint code.
    assert!(findings.is_empty());
}

#[test]
fn go_ast_runtime_finding_reports_ast_provenance() {
    use crate::rules::SignalSource;

    let file = facts(
        "src/domain/user.go",
        Some("Go"),
        "package domain\nfunc Parse() { panic(\"bad\") }\n",
    );

    let mut findings = LanguageRiskAudit.audit(&file, &ScanConfig::default());
    assert_eq!(findings.len(), 1);

    findings[0].populate_rule_metadata();
    assert_eq!(findings[0].provenance.signal_source, SignalSource::Ast);
}

#[test]
fn go_panic_inside_string_literal_is_not_flagged() {
    let file = facts(
        "src/domain/user.go",
        Some("Go"),
        "package domain\nfunc label() string { return \"panic(\" }\n",
    );

    let findings = LanguageRiskAudit.audit(&file, &ScanConfig::default());

    // The text `panic(` lives inside a string, not a call expression.
    assert!(findings.is_empty());
}

fn facts(path: &str, language: Option<&str>, content: &str) -> FileFacts {
    FileFacts {
        path: PathBuf::from(path),
        language: language.map(str::to_string),
        non_empty_lines: content.lines().count(),
        branch_count: 0,
        imports: Vec::new(),
        content: Some(content.to_string()),
        has_inline_tests: false,
    }
}
