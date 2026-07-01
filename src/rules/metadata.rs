use crate::findings::types::{Confidence, FindingCategory, Severity};
use crate::rules::requirements::RuleRequirements;
use crate::rules::{RuleLifecycle, SignalSource};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleMetadata {
    pub rule_id: &'static str,
    pub title: &'static str,
    pub category: FindingCategory,
    pub default_severity: Severity,
    /// Highest severity this rule may emit. Context (audit tiers, knowledge-pack
    /// upgrades) can raise a finding up to this ceiling but never above it;
    /// lowering below `default_severity` is always allowed (contextual demotion).
    /// Leave at the `DEFAULT` value to declare "never above `default_severity`".
    pub max_severity: Severity,
    pub default_confidence: Confidence,
    /// Highest confidence this rule may emit; see `max_severity` for semantics.
    pub max_confidence: Confidence,
    /// When false (the norm), every emitted finding carries exactly
    /// `default_confidence`. When true, the audit computes confidence per
    /// finding (capped at `confidence_ceiling()`).
    pub contextual_confidence: bool,
    pub lifecycle: RuleLifecycle,
    pub signal_source: SignalSource,
    /// Declarative execution contract used by documentation now and by the
    /// bounded scheduler/cache planner in later v0.20 milestones.
    pub requirements: RuleRequirements,
    pub docs_url: Option<&'static str>,
    pub description: &'static str,
    pub recommendation: Option<&'static str>,
    pub false_positive_notes: Option<&'static str>,
    pub tags: &'static [&'static str],
}

impl RuleMetadata {
    pub const DEFAULT: Self = Self {
        rule_id: "",
        title: "",
        category: FindingCategory::Architecture,
        default_severity: Severity::Info,
        max_severity: Severity::Info,
        default_confidence: Confidence::Medium,
        max_confidence: Confidence::Low,
        contextual_confidence: false,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::TextHeuristic,
        requirements: RuleRequirements::UNDECLARED,
        docs_url: None,
        description: "",
        recommendation: None,
        false_positive_notes: None,
        tags: &[],
    };

    pub fn severity_ceiling(&self) -> Severity {
        self.max_severity.max(self.default_severity)
    }

    pub fn confidence_ceiling(&self) -> Confidence {
        self.max_confidence.max(self.default_confidence)
    }
}
