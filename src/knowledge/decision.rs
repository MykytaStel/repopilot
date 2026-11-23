use crate::audits::context::{AuditContext, classify_file};
use crate::findings::provenance::{KnowledgeDecisionAction, KnowledgeDecisionProvenance};
use crate::findings::types::{Finding, Severity};
use crate::frameworks::DetectedFramework;
use crate::knowledge::active_knowledge;
use crate::knowledge::language::{language_id_for_name, profile_by_id};
use crate::knowledge::model::{
    RuleApplicability, RuleDecision, RuleDecisionAction, RuleMatchContext, RuleOverride,
    SupportLevel,
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

    if let Some(decision) = check_applicability(rule, context, trace) {
        return decision;
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

mod applicability;
mod context_api;
mod overlay;

use applicability::check_applicability;
pub use context_api::{
    apply_file_decision, apply_project_decisions, decide_for_audit_context,
    decide_for_audit_context_with_trace, decide_for_file, decide_for_file_with_trace,
    decide_for_project, record_decision_provenance,
};
use overlay::apply_overlay;
#[cfg(test)]
use overlay::apply_overlay_entries_with_marking;
#[cfg(test)]
pub(crate) use overlay::apply_overlay_for_test;

include!("decision/helpers.rs");

#[cfg(test)]
mod tests;
