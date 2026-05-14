use crate::audits::context::{AuditContext, classify_file};
use crate::findings::types::{Finding, Severity};
use crate::frameworks::DetectedFramework;
use crate::knowledge::bundled_knowledge;
use crate::knowledge::language::{language_id_for_name, profile_by_id};
use crate::knowledge::model::{RuleDecision, RuleDecisionAction, RuleMatchContext, RuleOverride};
use crate::scan::facts::{FileFacts, ScanFacts};
use crate::scan::path_classification::is_low_signal_audit_path;
use std::collections::HashSet;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SuppressReason {
    LanguageNotMatched,
    LanguageSupportTooLow,
    FrameworkNotMatched,
    RuntimeNotMatched,
    ParadigmNotMatched,
    LowSignalPath,
    ConfigFile,
    GeneratedFile,
}

impl fmt::Display for SuppressReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            SuppressReason::LanguageNotMatched => "rule does not apply to this language",
            SuppressReason::LanguageSupportTooLow => {
                "language support level is below this rule's minimum"
            }
            SuppressReason::FrameworkNotMatched => "rule does not apply to this framework",
            SuppressReason::RuntimeNotMatched => "rule does not apply to this runtime",
            SuppressReason::ParadigmNotMatched => "rule does not apply to this paradigm",
            SuppressReason::LowSignalPath => "low-signal audit path",
            SuppressReason::ConfigFile => "configuration file",
            SuppressReason::GeneratedFile => "generated file",
        };
        f.write_str(s)
    }
}

impl From<SuppressReason> for String {
    fn from(reason: SuppressReason) -> Self {
        reason.to_string()
    }
}

pub fn decide(context: &RuleMatchContext<'_>) -> RuleDecision {
    let Some(rule) = bundled_knowledge().rule_by_id(context.rule_id) else {
        return RuleDecision::apply(context.base_severity);
    };

    if !rule.languages.is_empty()
        && !context
            .languages
            .iter()
            .any(|l| rule.languages.contains(*l))
    {
        return RuleDecision::suppress(SuppressReason::LanguageNotMatched);
    }

    if let Some(minimum_support) = rule.minimum_support
        && !language_support_satisfies(rule, context.languages, minimum_support)
    {
        return RuleDecision::suppress(SuppressReason::LanguageSupportTooLow);
    }

    if !rule.frameworks.is_empty()
        && !context
            .frameworks
            .iter()
            .any(|f| rule.frameworks.contains(*f))
    {
        return RuleDecision::suppress(SuppressReason::FrameworkNotMatched);
    }

    if !rule.runtimes.is_empty() && !context.runtimes.iter().any(|r| rule.runtimes.contains(*r)) {
        return RuleDecision::suppress(SuppressReason::RuntimeNotMatched);
    }

    if !rule.paradigms.is_empty()
        && !context
            .paradigms
            .iter()
            .any(|p| rule.paradigms.contains(*p))
    {
        return RuleDecision::suppress(SuppressReason::ParadigmNotMatched);
    }

    if context.is_low_signal && rule.suppress_low_signal {
        return RuleDecision::suppress(SuppressReason::LowSignalPath);
    }

    if rule.suppress_config && context.roles.contains(&"config") {
        return RuleDecision::suppress(SuppressReason::ConfigFile);
    }

    if rule.suppress_generated && context.roles.contains(&"generated") {
        return RuleDecision::suppress(SuppressReason::GeneratedFile);
    }

    let mut decision = RuleDecision::apply(context.base_severity);

    for override_rule in &rule.overrides {
        if context.is_test && !is_test_override(override_rule) {
            continue;
        }
        if override_matches(override_rule, context) {
            decision = apply_override(override_rule, decision.severity);
            if decision.is_suppressed() {
                return decision;
            }
        }
    }

    decision
}

pub fn decide_for_audit_context(
    rule_id: &str,
    context: &AuditContext,
    base_severity: Severity,
    signal: Option<&str>,
) -> RuleDecision {
    let language_ids = [context.language_id()];
    decide_with_context(rule_id, &language_ids, context, false, base_severity, signal)
}

pub fn decide_for_file(
    rule_id: &str,
    file: &FileFacts,
    base_severity: Severity,
    signal: Option<&str>,
) -> RuleDecision {
    let context = classify_file(file);
    let mut language_ids = language_ids_for_file(file, &context);
    dedup_static_ids(&mut language_ids);
    let is_low_signal = is_low_signal_audit_path(&file.path) && !context.is_test;
    decide_with_context(rule_id, &language_ids, &context, is_low_signal, base_severity, signal)
}

fn decide_with_context(
    rule_id: &str,
    language_ids: &[&str],
    context: &AuditContext,
    is_low_signal: bool,
    base_severity: Severity,
    signal: Option<&str>,
) -> RuleDecision {
    let framework_ids = context.framework_ids();
    let role_ids = context.role_ids();
    let paradigm_ids = context.paradigm_ids();
    let runtime_ids = context.runtime_ids();

    decide(&RuleMatchContext {
        rule_id,
        languages: language_ids,
        frameworks: &framework_ids,
        roles: &role_ids,
        paradigms: &paradigm_ids,
        runtimes: &runtime_ids,
        is_test: context.is_test,
        is_low_signal,
        signal,
        base_severity,
    })
}

pub fn apply_file_decision(
    rule_id: &str,
    file: &FileFacts,
    mut finding: Finding,
    signal: Option<&str>,
) -> Option<Finding> {
    let decision = decide_for_file(rule_id, file, finding.severity, signal);
    if decision.is_suppressed() {
        return None;
    }
    finding.severity = decision.severity;
    Some(finding)
}

pub fn decide_for_project(
    rule_id: &str,
    facts: &ScanFacts,
    base_severity: Severity,
    signal: Option<&str>,
) -> RuleDecision {
    let mut language_ids = language_ids_for_project(facts);
    dedup_static_ids(&mut language_ids);
    let framework_ids = framework_ids_for_project(facts);

    decide(&RuleMatchContext {
        rule_id,
        languages: &language_ids,
        frameworks: &framework_ids,
        roles: &[],
        paradigms: &[],
        runtimes: &[],
        is_test: false,
        is_low_signal: false,
        signal,
        base_severity,
    })
}

pub fn apply_project_decisions(facts: &ScanFacts, findings: Vec<Finding>) -> Vec<Finding> {
    findings
        .into_iter()
        .filter_map(|mut finding| {
            let decision = decide_for_project(&finding.rule_id, facts, finding.severity, None);
            if decision.is_suppressed() {
                return None;
            }
            finding.severity = decision.severity;
            Some(finding)
        })
        .collect()
}

fn override_matches(override_rule: &RuleOverride, context: &RuleMatchContext<'_>) -> bool {
    if override_rule
        .signal
        .as_deref()
        .is_some_and(|signal| Some(signal) != context.signal)
    {
        return false;
    }

    if override_rule
        .language
        .as_deref()
        .is_some_and(|language| !context.languages.contains(&language))
    {
        return false;
    }

    if override_rule
        .framework
        .as_deref()
        .is_some_and(|framework| !context.frameworks.contains(&framework))
    {
        return false;
    }

    if override_rule
        .runtime
        .as_deref()
        .is_some_and(|runtime| !context.runtimes.contains(&runtime))
    {
        return false;
    }

    if override_rule
        .paradigm
        .as_deref()
        .is_some_and(|paradigm| !context.paradigms.contains(&paradigm))
    {
        return false;
    }

    if override_rule
        .role
        .as_deref()
        .is_some_and(|role| !context.roles.contains(&role))
    {
        return false;
    }

    true
}

fn is_test_override(override_rule: &RuleOverride) -> bool {
    matches!(override_rule.role.as_deref(), Some("test" | "rust-test"))
}

fn apply_override(override_rule: &RuleOverride, current: Severity) -> RuleDecision {
    match override_rule.action {
        RuleDecisionAction::Apply => RuleDecision {
            action: RuleDecisionAction::Apply,
            severity: override_rule.severity.unwrap_or(current),
            reason: override_rule.reason.clone(),
        },
        RuleDecisionAction::Suppress => RuleDecision {
            action: RuleDecisionAction::Suppress,
            severity: Severity::Info,
            reason: override_rule.reason.clone(),
        },
        RuleDecisionAction::Downgrade => {
            let severity = override_rule
                .severity
                .filter(|severity| *severity < current)
                .unwrap_or(current);
            RuleDecision {
                action: RuleDecisionAction::Downgrade,
                severity,
                reason: override_rule.reason.clone(),
            }
        }
        RuleDecisionAction::Upgrade => {
            let severity = override_rule
                .severity
                .filter(|severity| *severity > current)
                .unwrap_or(current);
            RuleDecision {
                action: RuleDecisionAction::Upgrade,
                severity,
                reason: override_rule.reason.clone(),
            }
        }
    }
}

fn language_support_satisfies(
    rule: &crate::knowledge::model::RuleApplicability,
    context_languages: &[&str],
    minimum_support: crate::knowledge::model::SupportLevel,
) -> bool {
    context_languages.iter().any(|language| {
        (rule.languages.is_empty() || rule.languages.contains(*language))
            && profile_by_id(language).is_some_and(|profile| profile.support >= minimum_support)
    })
}

fn language_ids_for_file(file: &FileFacts, context: &AuditContext) -> Vec<&'static str> {
    let mut languages = Vec::new();
    if let Some(language) = file
        .language
        .as_deref()
        .and_then(language_id_for_name)
        .filter(|language| *language != "unknown")
    {
        languages.push(language);
    }

    let context_language = context.language_id();
    if context_language != "unknown" {
        languages.push(context_language);
    }

    languages
}

fn language_ids_for_project(facts: &ScanFacts) -> Vec<&'static str> {
    let mut languages = facts
        .languages
        .iter()
        .filter_map(|language| language_id_for_name(&language.name))
        .collect::<Vec<_>>();

    if languages.is_empty() {
        languages.extend(
            facts
                .files
                .iter()
                .filter_map(|file| file.language.as_deref())
                .filter_map(language_id_for_name),
        );
    }

    languages
}

fn framework_ids_for_project(facts: &ScanFacts) -> Vec<&'static str> {
    let mut frameworks = facts
        .detected_frameworks
        .iter()
        .filter_map(framework_id)
        .collect::<Vec<_>>();

    for project in &facts.framework_projects {
        frameworks.extend(project.frameworks.iter().filter_map(framework_id));
    }

    dedup_static_ids(&mut frameworks);
    frameworks
}

fn framework_id(framework: &DetectedFramework) -> Option<&'static str> {
    match framework {
        DetectedFramework::ReactNative { .. } => Some("react-native"),
        DetectedFramework::Expo { .. } => Some("expo"),
        DetectedFramework::NextJs { .. } => Some("nextjs"),
        DetectedFramework::React { .. } => Some("react"),
        DetectedFramework::Vue { .. } => Some("vue"),
        DetectedFramework::Angular { .. } => Some("angular"),
        DetectedFramework::Svelte { .. } => Some("svelte"),
        DetectedFramework::NestJs { .. } => Some("nestjs"),
        DetectedFramework::Express { .. } => Some("express"),
        DetectedFramework::Django { .. } => Some("django"),
        DetectedFramework::Flask { .. } => Some("flask"),
        DetectedFramework::FastApi { .. } => Some("fastapi"),
        DetectedFramework::Gin { .. } => Some("gin"),
        DetectedFramework::Echo { .. } => Some("echo"),
        DetectedFramework::Fiber { .. } => Some("fiber"),
    }
}

fn dedup_static_ids(values: &mut Vec<&'static str>) {
    let mut seen = HashSet::new();
    values.retain(|value| seen.insert(*value));
}

#[cfg(test)]
mod tests {
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
}
