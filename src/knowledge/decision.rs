use crate::audits::context::{AuditContext, classify_file};
use crate::findings::types::{Finding, Severity};
use crate::frameworks::DetectedFramework;
use crate::knowledge::active_knowledge;
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
    let Some(rule) = active_knowledge().rule_by_id(context.rule_id) else {
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

    let mut decision =
        RuleDecision::apply(context.base_severity).with_risk_signal(rule.risk.clone());

    for override_rule in &rule.overrides {
        if context.is_test && !is_test_override(override_rule) {
            continue;
        }
        if override_matches(override_rule, context) {
            decision = apply_override(override_rule, decision.severity, rule.risk.clone());
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
    decide_with_context(
        rule_id,
        &language_ids,
        context,
        false,
        base_severity,
        signal,
    )
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
    decide_with_context(
        rule_id,
        &language_ids,
        &context,
        is_low_signal,
        base_severity,
        signal,
    )
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

include!("decision/helpers.rs");

#[cfg(test)]
mod tests;
