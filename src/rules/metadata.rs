use crate::findings::types::{Confidence, FindingCategory, Severity};
use crate::rules::{RuleLifecycle, SignalSource};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleMetadata {
    pub rule_id: &'static str,
    pub title: &'static str,
    pub category: FindingCategory,
    pub default_severity: Severity,
    pub default_confidence: Confidence,
    pub lifecycle: RuleLifecycle,
    pub signal_source: SignalSource,
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
        default_confidence: Confidence::Medium,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::TextHeuristic,
        docs_url: None,
        description: "",
        recommendation: None,
        false_positive_notes: None,
        tags: &[],
    };
}
