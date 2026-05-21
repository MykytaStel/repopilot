use super::*;
use crate::audits::context::{
    AuditContext, FileRole, LanguageKind, ProgrammingParadigm, RuntimeKind,
};

#[test]
fn suppresses_rust_unwrap_in_tests() {
    let context = rust_context(vec![FileRole::Test], vec![RuntimeKind::RustLibrary]);

    let decision = decide_for_audit_context(
        "language.rust.panic-risk",
        &context,
        Severity::Medium,
        Some("rust.unwrap"),
    );

    assert!(decision.is_suppressed());
}

#[test]
fn upgrades_rust_panic_in_domain_code() {
    let context = rust_context(vec![FileRole::Domain], vec![RuntimeKind::RustLibrary]);

    let decision = decide_for_audit_context(
        "language.rust.panic-risk",
        &context,
        Severity::Medium,
        Some("rust.panic"),
    );

    assert_eq!(decision.action, RuleDecisionAction::Upgrade);
    assert_eq!(decision.severity, Severity::High);
}

#[test]
fn downgrades_rust_unwrap_at_cli_boundary() {
    let context = rust_context(Vec::new(), vec![RuntimeKind::RustCli]);

    let decision = decide_for_audit_context(
        "language.rust.panic-risk",
        &context,
        Severity::Medium,
        Some("rust.unwrap"),
    );

    assert_eq!(decision.action, RuleDecisionAction::Downgrade);
    assert_eq!(decision.severity, Severity::Low);
}

#[test]
fn functional_paradigm_does_not_suppress_or_create_a_problem() {
    let context = AuditContext {
        language: LanguageKind::Rust,
        frameworks: Vec::new(),
        roles: vec![FileRole::Domain],
        paradigms: vec![ProgrammingParadigm::Functional],
        runtimes: vec![RuntimeKind::RustLibrary],
        is_test: false,
    };

    let decision = decide_for_audit_context(
        "code-quality.complex-file",
        &context,
        Severity::Medium,
        None,
    );

    assert_eq!(decision.action, RuleDecisionAction::Apply);
    assert_eq!(decision.severity, Severity::Medium);
}

#[test]
fn suppresses_rust_rule_for_python_context() {
    let context = AuditContext {
        language: LanguageKind::Python,
        frameworks: Vec::new(),
        roles: Vec::new(),
        paradigms: vec![ProgrammingParadigm::Unknown],
        runtimes: Vec::new(),
        is_test: false,
    };

    let decision =
        decide_for_audit_context("language.rust.panic-risk", &context, Severity::Medium, None);

    assert!(decision.is_suppressed());
}

#[test]
fn suppresses_react_native_rule_for_plain_react_context() {
    use crate::audits::context::FrameworkKind;

    let context = AuditContext {
        language: LanguageKind::TypeScript,
        frameworks: vec![FrameworkKind::React],
        roles: Vec::new(),
        paradigms: Vec::new(),
        runtimes: Vec::new(),
        is_test: false,
    };

    let decision = decide_for_audit_context(
        "framework.react-native.inline-style",
        &context,
        Severity::Medium,
        None,
    );

    assert!(decision.is_suppressed());
}

#[test]
fn applies_rule_when_no_knowledge_entry_exists() {
    let decision = decide_for_audit_context(
        "nonexistent.unknown.rule",
        &rust_context(Vec::new(), Vec::new()),
        Severity::High,
        None,
    );

    assert_eq!(decision.action, RuleDecisionAction::Apply);
    assert_eq!(decision.severity, Severity::High);
}

#[test]
fn suppresses_low_signal_for_rules_with_flag() {
    let decision = decide(&RuleMatchContext {
        rule_id: "language.rust.panic-risk",
        languages: &["rust"],
        frameworks: &[],
        roles: &[],
        paradigms: &[],
        runtimes: &[],
        is_test: false,
        is_low_signal: true,
        signal: None,
        base_severity: Severity::Medium,
    });

    assert!(decision.is_suppressed());
}

fn rust_context(roles: Vec<FileRole>, runtimes: Vec<RuntimeKind>) -> AuditContext {
    AuditContext {
        language: LanguageKind::Rust,
        frameworks: Vec::new(),
        roles,
        paradigms: vec![ProgrammingParadigm::Unknown],
        runtimes,
        is_test: false,
    }
}
