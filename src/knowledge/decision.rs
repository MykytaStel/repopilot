use crate::audits::context::{AuditContext, classify_file};
use crate::findings::provenance::{KnowledgeDecisionAction, KnowledgeDecisionProvenance};
use crate::findings::types::{Finding, Severity};
use crate::frameworks::DetectedFramework;
use crate::knowledge::active_knowledge;
use crate::knowledge::language::{language_id_for_name, profile_by_id};
use crate::knowledge::model::{
    RuleDecision, RuleDecisionAction, RuleMatchContext, RuleOverride, SupportLevel,
};
use crate::scan::facts::{FileFacts, ScanFacts};
use crate::scan::path_classification::is_low_signal_audit_path;
use std::collections::HashSet;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionTraceStage {
    RuleLookup,
    Applicability,
    BaseDecision,
    Override,
    Overlay,
}

impl DecisionTraceStage {
    pub fn as_id(self) -> &'static str {
        match self {
            DecisionTraceStage::RuleLookup => "rule-lookup",
            DecisionTraceStage::Applicability => "applicability",
            DecisionTraceStage::BaseDecision => "base-decision",
            DecisionTraceStage::Override => "override",
            DecisionTraceStage::Overlay => "overlay",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionTraceStatus {
    Applied,
    Passed,
    Failed,
    Matched,
    NotMatched,
    Skipped,
}

impl DecisionTraceStatus {
    pub fn as_id(self) -> &'static str {
        match self {
            DecisionTraceStatus::Applied => "applied",
            DecisionTraceStatus::Passed => "passed",
            DecisionTraceStatus::Failed => "failed",
            DecisionTraceStatus::Matched => "matched",
            DecisionTraceStatus::NotMatched => "not-matched",
            DecisionTraceStatus::Skipped => "skipped",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionTraceStep {
    pub stage: DecisionTraceStage,
    pub status: DecisionTraceStatus,
    pub label: String,
    pub criteria: Vec<String>,
    pub action: Option<RuleDecisionAction>,
    pub severity_before: Severity,
    pub severity_after: Severity,
    pub reason: String,
    pub override_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleDecisionTrace {
    pub decision: RuleDecision,
    pub steps: Vec<DecisionTraceStep>,
}

/// Optional trace sink for the shared decision evaluator.
///
/// Scan/review decisions use a disabled recorder, so trace labels, criteria, and
/// reasons are not allocated. Explicit explain APIs enable the recorder while
/// executing the same evaluator.
struct TraceRecorder<'a> {
    steps: Option<&'a mut Vec<DecisionTraceStep>>,
}

impl<'a> TraceRecorder<'a> {
    fn disabled() -> Self {
        Self { steps: None }
    }

    fn enabled(steps: &'a mut Vec<DecisionTraceStep>) -> Self {
        Self { steps: Some(steps) }
    }

    fn push<F>(&mut self, build: F)
    where
        F: FnOnce() -> DecisionTraceStep,
    {
        if let Some(steps) = self.steps.as_mut() {
            (*steps).push(build());
        }
    }
}

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
    let mut trace = TraceRecorder::disabled();
    decide_internal(context, &mut trace)
}

pub fn decide_with_trace(context: &RuleMatchContext<'_>) -> RuleDecisionTrace {
    let mut steps = Vec::new();
    let decision = {
        let mut trace = TraceRecorder::enabled(&mut steps);
        decide_internal(context, &mut trace)
    };
    RuleDecisionTrace { decision, steps }
}

fn decide_internal(context: &RuleMatchContext<'_>, trace: &mut TraceRecorder<'_>) -> RuleDecision {
    let Some(rule) = active_knowledge().rule_by_id(context.rule_id) else {
        let decision = RuleDecision::apply(context.base_severity);
        trace.push(|| DecisionTraceStep {
            stage: DecisionTraceStage::RuleLookup,
            status: DecisionTraceStatus::NotMatched,
            label: "knowledge-entry".to_string(),
            criteria: vec![format!("rule_id={}", context.rule_id)],
            action: Some(RuleDecisionAction::Apply),
            severity_before: context.base_severity,
            severity_after: context.base_severity,
            reason: "no bundled knowledge entry; preserve the supplied base severity".to_string(),
            override_index: None,
        });
        return decision;
    };

    trace.push(|| DecisionTraceStep {
        stage: DecisionTraceStage::RuleLookup,
        status: DecisionTraceStatus::Matched,
        label: "knowledge-entry".to_string(),
        criteria: vec![format!("rule_id={}", context.rule_id)],
        action: None,
        severity_before: context.base_severity,
        severity_after: context.base_severity,
        reason: "bundled knowledge entry found".to_string(),
        override_index: None,
    });

    if !rule.languages.is_empty() {
        let passed = context
            .languages
            .iter()
            .any(|language| rule.languages.contains(*language));
        if let Some(decision) = record_applicability(
            trace,
            "language",
            || {
                vec![
                    format!("allowed={}", sorted_values(&rule.languages)),
                    format!("actual={}", joined_ids(context.languages)),
                ]
            },
            passed,
            SuppressReason::LanguageNotMatched,
            context.base_severity,
        ) {
            return decision;
        }
    }

    if let Some(minimum_support) = rule.minimum_support {
        let passed = language_support_satisfies(rule, context.languages, minimum_support);
        if let Some(decision) = record_applicability(
            trace,
            "language-support",
            || {
                vec![
                    format!("minimum={}", support_level_id(minimum_support)),
                    format!("languages={}", joined_ids(context.languages)),
                ]
            },
            passed,
            SuppressReason::LanguageSupportTooLow,
            context.base_severity,
        ) {
            return decision;
        }
    }

    if !rule.frameworks.is_empty() {
        let passed = context
            .frameworks
            .iter()
            .any(|framework| rule.frameworks.contains(*framework));
        if let Some(decision) = record_applicability(
            trace,
            "framework",
            || {
                vec![
                    format!("allowed={}", sorted_values(&rule.frameworks)),
                    format!("actual={}", joined_ids(context.frameworks)),
                ]
            },
            passed,
            SuppressReason::FrameworkNotMatched,
            context.base_severity,
        ) {
            return decision;
        }
    }

    if !rule.runtimes.is_empty() {
        let passed = context
            .runtimes
            .iter()
            .any(|runtime| rule.runtimes.contains(*runtime));
        if let Some(decision) = record_applicability(
            trace,
            "runtime",
            || {
                vec![
                    format!("allowed={}", sorted_values(&rule.runtimes)),
                    format!("actual={}", joined_ids(context.runtimes)),
                ]
            },
            passed,
            SuppressReason::RuntimeNotMatched,
            context.base_severity,
        ) {
            return decision;
        }
    }

    if !rule.paradigms.is_empty() {
        let passed = context
            .paradigms
            .iter()
            .any(|paradigm| rule.paradigms.contains(*paradigm));
        if let Some(decision) = record_applicability(
            trace,
            "paradigm",
            || {
                vec![
                    format!("allowed={}", sorted_values(&rule.paradigms)),
                    format!("actual={}", joined_ids(context.paradigms)),
                ]
            },
            passed,
            SuppressReason::ParadigmNotMatched,
            context.base_severity,
        ) {
            return decision;
        }
    }

    if rule.suppress_low_signal
        && let Some(decision) = record_applicability(
            trace,
            "low-signal-path",
            || vec![format!("is_low_signal={}", context.is_low_signal)],
            !context.is_low_signal,
            SuppressReason::LowSignalPath,
            context.base_severity,
        )
    {
        return decision;
    }

    if rule.suppress_config {
        let is_config = context.roles.contains(&"config");
        if let Some(decision) = record_applicability(
            trace,
            "config-role",
            || vec![format!("roles={}", joined_ids(context.roles))],
            !is_config,
            SuppressReason::ConfigFile,
            context.base_severity,
        ) {
            return decision;
        }
    }

    if rule.suppress_generated {
        let is_generated = context.roles.contains(&"generated");
        if let Some(decision) = record_applicability(
            trace,
            "generated-role",
            || vec![format!("roles={}", joined_ids(context.roles))],
            !is_generated,
            SuppressReason::GeneratedFile,
            context.base_severity,
        ) {
            return decision;
        }
    }

    let mut decision =
        RuleDecision::apply(context.base_severity).with_risk_signal(rule.risk.clone());
    trace.push(|| DecisionTraceStep {
        stage: DecisionTraceStage::BaseDecision,
        status: DecisionTraceStatus::Applied,
        label: "base-severity".to_string(),
        criteria: vec![format!("base={}", context.base_severity.label())],
        action: Some(RuleDecisionAction::Apply),
        severity_before: context.base_severity,
        severity_after: context.base_severity,
        reason: if rule.risk.is_some() {
            "base severity retained and the rule-level risk signal attached".to_string()
        } else {
            "base severity retained before ordered overrides".to_string()
        },
        override_index: None,
    });

    for (index, override_rule) in rule.overrides.iter().enumerate() {
        let before = decision.severity;

        if context.is_test && !is_test_override(override_rule) {
            trace.push(|| DecisionTraceStep {
                stage: DecisionTraceStage::Override,
                status: DecisionTraceStatus::Skipped,
                label: format!("override[{index}]"),
                criteria: override_criteria(override_rule),
                action: Some(override_rule.action),
                severity_before: before,
                severity_after: before,
                reason: "non-test override skipped because the file is test context".to_string(),
                override_index: Some(index),
            });
            continue;
        }

        if !override_matches(override_rule, context) {
            trace.push(|| DecisionTraceStep {
                stage: DecisionTraceStage::Override,
                status: DecisionTraceStatus::NotMatched,
                label: format!("override[{index}]"),
                criteria: override_criteria(override_rule),
                action: Some(override_rule.action),
                severity_before: before,
                severity_after: before,
                reason: "override criteria did not all match".to_string(),
                override_index: Some(index),
            });
            continue;
        }

        decision = apply_override(override_rule, before, rule.risk.clone());
        trace.push(|| DecisionTraceStep {
            stage: DecisionTraceStage::Override,
            status: DecisionTraceStatus::Applied,
            label: format!("override[{index}]"),
            criteria: override_criteria(override_rule),
            action: Some(override_rule.action),
            severity_before: before,
            severity_after: decision.severity,
            reason: decision
                .reason
                .clone()
                .unwrap_or_else(|| format!("override[{index}] matched")),
            override_index: Some(index),
        });
        if decision.is_suppressed() {
            return decision;
        }
    }

    // The overlay stage below is only unreachable for rule ids with no bundled
    // knowledge entry (see the early return at the top of this function). That
    // cannot happen for any overlay entry: overlay validation
    // (`src/knowledge/overlay/mod.rs::build_entry`) already rejects unregistered
    // rule ids, and every registered rule id is guaranteed a knowledge entry by
    // `knowledge::loader::tests::all_registered_rules_have_knowledge_applicability`.
    decision = apply_overlay(context, decision, trace);

    decision
}

fn apply_overlay(
    context: &RuleMatchContext<'_>,
    decision: RuleDecision,
    trace: &mut TraceRecorder<'_>,
) -> RuleDecision {
    apply_overlay_entries(
        context,
        crate::knowledge::overlay::active_overlay().entries(),
        decision,
        trace,
    )
}

fn apply_overlay_entries(
    context: &RuleMatchContext<'_>,
    entries: &[crate::knowledge::overlay::OverlayEntry],
    mut decision: RuleDecision,
    trace: &mut TraceRecorder<'_>,
) -> RuleDecision {
    use crate::knowledge::overlay::OverlayTarget;

    for entry in entries {
        let OverlayTarget::Rule(rule_id) = &entry.target else {
            continue;
        };
        if rule_id != context.rule_id {
            continue;
        }
        if let Some(expires) = entry.expires
            && expires < chrono::Utc::now().date_naive()
        {
            continue;
        }
        let path_matches = match (&entry.path_glob, context.path) {
            (None, _) => true,
            (Some(glob), Some(path)) => glob.is_match(path),
            (Some(_), None) => false,
        };
        if !path_matches {
            continue;
        }

        let before = decision.severity;
        decision = match entry.severity {
            Some(severity) => RuleDecision {
                action: severity_transition_action(before, severity),
                severity,
                reason: entry.reason.clone(),
                risk_signal: decision.risk_signal.clone(),
                via_overlay: true,
            },
            None => RuleDecision::suppress(
                entry
                    .reason
                    .clone()
                    .unwrap_or_else(|| format!("overlay[{}] suppresses this rule", entry.index)),
            )
            .with_via_overlay(true),
        };

        trace.push(|| DecisionTraceStep {
            stage: DecisionTraceStage::Overlay,
            status: DecisionTraceStatus::Applied,
            label: format!("overlay[{}]", entry.index),
            criteria: vec![
                format!("rule={rule_id}"),
                format!(
                    "path={}",
                    entry.path_text.clone().unwrap_or_else(|| "*".to_string())
                ),
            ],
            action: Some(decision.action),
            severity_before: before,
            severity_after: decision.severity,
            reason: decision
                .reason
                .clone()
                .unwrap_or_else(|| format!("overlay[{}] matched", entry.index)),
            override_index: None,
        });

        if decision.is_suppressed() {
            return decision;
        }
    }

    decision
}

#[cfg(test)]
pub(crate) fn apply_overlay_for_test(
    context: &RuleMatchContext<'_>,
    entries: &[crate::knowledge::overlay::OverlayEntry],
) -> RuleDecision {
    let mut trace = TraceRecorder::disabled();
    let decision = RuleDecision::apply(context.base_severity);
    apply_overlay_entries(context, entries, decision, &mut trace)
}

fn record_applicability<F>(
    trace: &mut TraceRecorder<'_>,
    label: &'static str,
    criteria: F,
    passed: bool,
    failure: SuppressReason,
    severity: Severity,
) -> Option<RuleDecision>
where
    F: FnOnce() -> Vec<String>,
{
    let decision = if passed {
        None
    } else {
        Some(RuleDecision::suppress(failure))
    };
    let action = decision.as_ref().map(|_| RuleDecisionAction::Suppress);
    let severity_after = decision.as_ref().map_or(severity, |value| value.severity);

    trace.push(|| DecisionTraceStep {
        stage: DecisionTraceStage::Applicability,
        status: if passed {
            DecisionTraceStatus::Passed
        } else {
            DecisionTraceStatus::Failed
        },
        label: label.to_string(),
        criteria: criteria(),
        action,
        severity_before: severity,
        severity_after,
        reason: if passed {
            format!("{label} applicability passed")
        } else {
            failure.to_string()
        },
        override_index: None,
    });
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
        None,
    )
}

pub fn decide_for_audit_context_with_trace(
    rule_id: &str,
    context: &AuditContext,
    base_severity: Severity,
    signal: Option<&str>,
) -> RuleDecisionTrace {
    let language_ids = [context.language_id()];
    decide_with_context_trace(
        rule_id,
        &language_ids,
        context,
        false,
        base_severity,
        signal,
        None,
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
    let path = file.path.to_str();
    decide_with_context(
        rule_id,
        &language_ids,
        &context,
        is_low_signal,
        base_severity,
        signal,
        path,
    )
}

pub fn decide_for_file_with_trace(
    rule_id: &str,
    file: &FileFacts,
    base_severity: Severity,
    signal: Option<&str>,
) -> RuleDecisionTrace {
    let context = classify_file(file);
    let mut language_ids = language_ids_for_file(file, &context);
    dedup_static_ids(&mut language_ids);
    let is_low_signal = is_low_signal_audit_path(&file.path) && !context.is_test;
    let path = file.path.to_str();
    decide_with_context_trace(
        rule_id,
        &language_ids,
        &context,
        is_low_signal,
        base_severity,
        signal,
        path,
    )
}

fn decide_with_context(
    rule_id: &str,
    language_ids: &[&str],
    context: &AuditContext,
    is_low_signal: bool,
    base_severity: Severity,
    signal: Option<&str>,
    path: Option<&str>,
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
        path,
    })
}

fn decide_with_context_trace(
    rule_id: &str,
    language_ids: &[&str],
    context: &AuditContext,
    is_low_signal: bool,
    base_severity: Severity,
    signal: Option<&str>,
    path: Option<&str>,
) -> RuleDecisionTrace {
    let framework_ids = context.framework_ids();
    let role_ids = context.role_ids();
    let paradigm_ids = context.paradigm_ids();
    let runtime_ids = context.runtime_ids();
    decide_with_trace(&RuleMatchContext {
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
        path,
    })
}

pub fn apply_file_decision(
    rule_id: &str,
    file: &FileFacts,
    mut finding: Finding,
    signal: Option<&str>,
) -> Option<Finding> {
    let base_severity = finding.severity;
    let decision = decide_for_file(rule_id, file, base_severity, signal);
    if decision.is_suppressed() {
        return None;
    }
    record_decision_provenance(&mut finding, base_severity, signal, &decision);
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
        path: None,
    })
}

pub fn apply_project_decisions(facts: &ScanFacts, findings: Vec<Finding>) -> Vec<Finding> {
    findings
        .into_iter()
        .filter_map(|mut finding| {
            let base_severity = finding.severity;
            let decision = decide_for_project(&finding.rule_id, facts, base_severity, None);
            if decision.is_suppressed() {
                return None;
            }
            record_decision_provenance(&mut finding, base_severity, None, &decision);
            finding.severity = decision.severity;
            Some(finding)
        })
        .collect()
}

pub fn record_decision_provenance(
    finding: &mut Finding,
    base_severity: Severity,
    signal: Option<&str>,
    decision: &RuleDecision,
) {
    finding.provenance.knowledge_decision = Some(KnowledgeDecisionProvenance {
        base_severity,
        signal: signal.map(str::to_string),
        action: match decision.action {
            RuleDecisionAction::Apply => KnowledgeDecisionAction::Apply,
            RuleDecisionAction::Suppress => KnowledgeDecisionAction::Suppress,
            RuleDecisionAction::Downgrade => KnowledgeDecisionAction::Downgrade,
            RuleDecisionAction::Upgrade => KnowledgeDecisionAction::Upgrade,
        },
        decided_severity: decision.severity,
        reason: decision.reason.clone(),
    });
}

include!("decision/helpers.rs");

#[cfg(test)]
mod tests;
