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
        path: None,
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

#[test]
fn trace_preserves_ordered_matching_overrides_and_severity_transitions() {
    let context = AuditContext {
        language: LanguageKind::Kotlin,
        frameworks: Vec::new(),
        roles: vec![FileRole::Domain, FileRole::TestSupport],
        paradigms: vec![ProgrammingParadigm::ObjectOriented],
        runtimes: vec![RuntimeKind::Jvm],
        is_test: false,
    };
    let trace = decide_for_audit_context_with_trace(
        "language.managed.fatal-exception-risk",
        &context,
        Severity::Medium,
        None,
    );
    assert_eq!(trace.decision.action, RuleDecisionAction::Downgrade);
    assert_eq!(trace.decision.severity, Severity::Low);
    let applied = trace
        .steps
        .iter()
        .filter(|step| {
            step.stage == DecisionTraceStage::Override
                && step.status == DecisionTraceStatus::Applied
        })
        .collect::<Vec<_>>();
    assert_eq!(applied.len(), 2);
    assert_eq!(applied[0].override_index, Some(1));
    assert_eq!(applied[0].severity_before, Severity::Medium);
    assert_eq!(applied[0].severity_after, Severity::High);
    assert_eq!(applied[1].override_index, Some(2));
    assert_eq!(applied[1].severity_before, Severity::High);
    assert_eq!(applied[1].severity_after, Severity::Low);
}

#[test]
fn trace_records_the_applicability_check_that_suppressed_a_rule() {
    let context = AuditContext {
        language: LanguageKind::Python,
        frameworks: Vec::new(),
        roles: Vec::new(),
        paradigms: vec![ProgrammingParadigm::Unknown],
        runtimes: vec![RuntimeKind::Python],
        is_test: false,
    };
    let trace = decide_for_audit_context_with_trace(
        "language.rust.panic-risk",
        &context,
        Severity::Medium,
        Some("rust.panic"),
    );
    assert!(trace.decision.is_suppressed());
    let failed = trace
        .steps
        .iter()
        .find(|step| step.status == DecisionTraceStatus::Failed)
        .expect("failed applicability step");
    assert_eq!(failed.stage, DecisionTraceStage::Applicability);
    assert_eq!(failed.label, "language");
    assert_eq!(failed.action, Some(RuleDecisionAction::Suppress));
}

#[test]
fn traced_and_compatibility_decisions_remain_identical() {
    let context = rust_context(vec![FileRole::Domain], vec![RuntimeKind::RustLibrary]);
    let compatibility = decide_for_audit_context(
        "language.rust.panic-risk",
        &context,
        Severity::Medium,
        Some("rust.panic"),
    );
    let traced = decide_for_audit_context_with_trace(
        "language.rust.panic-risk",
        &context,
        Severity::Medium,
        Some("rust.panic"),
    );
    assert_eq!(compatibility, traced.decision);
}

#[test]
fn disabled_trace_recorder_does_not_materialize_steps() {
    let mut materialized = false;
    let mut recorder = TraceRecorder::disabled();

    recorder.push(|| {
        materialized = true;
        DecisionTraceStep {
            stage: DecisionTraceStage::RuleLookup,
            status: DecisionTraceStatus::Matched,
            label: "should-not-be-built".to_string(),
            criteria: vec!["should-not-be-built".to_string()],
            action: None,
            severity_before: Severity::Info,
            severity_after: Severity::Info,
            reason: "should-not-be-built".to_string(),
            override_index: None,
        }
    });

    assert!(!materialized);
}

#[test]
fn applied_file_decision_preserves_replayable_provenance() {
    use crate::findings::provenance::KnowledgeDecisionAction;
    use crate::findings::types::{Evidence, FindingCategory};
    use std::path::PathBuf;

    let file = FileFacts {
        path: PathBuf::from("src/domain/service.rs"),
        language: Some("Rust".to_string()),
        non_empty_lines: 1,
        branch_count: 0,
        imports: Vec::new(),
        content: Some("pub fn run() { panic!(\"boom\"); }\n".to_string()),
        has_inline_tests: false,
        in_executable_package: false,
        deferred_imports: Vec::new(),
    };
    let base_severity = Severity::Medium;
    let signal = "rust.panic";
    let expected = decide_for_file(
        "language.rust.panic-risk",
        &file,
        base_severity,
        Some(signal),
    );
    assert!(!expected.is_suppressed());

    let finding = Finding {
        rule_id: "language.rust.panic-risk".to_string(),
        category: FindingCategory::CodeQuality,
        severity: base_severity,
        evidence: vec![Evidence {
            path: file.path.clone(),
            line_start: 1,
            line_end: None,
            snippet: "panic!(\"boom\")".to_string(),
        }],
        ..Finding::default()
    };

    let mut applied = apply_file_decision("language.rust.panic-risk", &file, finding, Some(signal))
        .expect("finding should remain visible");
    applied.populate_rule_metadata();

    let provenance = applied
        .provenance
        .knowledge_decision
        .as_ref()
        .expect("knowledge decision provenance");
    assert_eq!(provenance.base_severity, base_severity);
    assert_eq!(provenance.signal.as_deref(), Some(signal));
    assert_eq!(provenance.decided_severity, expected.severity);
    assert_eq!(
        provenance.action,
        match expected.action {
            RuleDecisionAction::Apply => KnowledgeDecisionAction::Apply,
            RuleDecisionAction::Suppress => KnowledgeDecisionAction::Suppress,
            RuleDecisionAction::Downgrade => KnowledgeDecisionAction::Downgrade,
            RuleDecisionAction::Upgrade => KnowledgeDecisionAction::Upgrade,
        }
    );
    assert_eq!(applied.provenance.detector, "language.rust.panic-risk");

    let json = serde_json::to_value(&applied).expect("serialize finding");
    assert_eq!(json["provenance"]["knowledge_decision"]["signal"], signal);
    assert_eq!(
        json["provenance"]["knowledge_decision"]["base_severity"],
        base_severity.label()
    );
}

#[test]
fn default_provenance_omits_absent_knowledge_decision() {
    let value = serde_json::to_value(crate::findings::provenance::FindingProvenance::default())
        .expect("serialize provenance");
    assert!(value.get("knowledge_decision").is_none());
}
