use crate::rules::{
    RuleLifecycle, RuleMetadata, SignalSource, all_rule_metadata, lookup_rule_metadata,
};
use serde::Serialize;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RuleCatalogFilter {
    pub lifecycle: Option<RuleLifecycle>,
    pub source: Option<SignalSource>,
}

#[derive(Debug, Serialize)]
pub struct RuleCatalogReport {
    pub rules: Vec<RuleCatalogItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuleCatalogItem {
    pub rule_id: &'static str,
    pub title: &'static str,
    pub category: String,
    pub severity: String,
    pub confidence: String,
    pub lifecycle: RuleLifecycle,
    pub signal_source: SignalSource,
    pub docs_url: Option<&'static str>,
    pub tags: &'static [&'static str],
    pub description: &'static str,
    pub recommendation: Option<&'static str>,
    pub false_positive_notes: Option<&'static str>,
}

pub fn list_rule_catalog(filter: RuleCatalogFilter) -> RuleCatalogReport {
    let rules = all_rule_metadata()
        .filter(|rule| filter.lifecycle.is_none_or(|value| rule.lifecycle == value))
        .filter(|rule| {
            filter
                .source
                .is_none_or(|value| rule.signal_source == value)
        })
        .map(RuleCatalogItem::from)
        .collect::<Vec<_>>();

    RuleCatalogReport { rules }
}

pub fn inspect_rule(rule_id: &str) -> Option<RuleCatalogItem> {
    lookup_rule_metadata(rule_id).map(RuleCatalogItem::from)
}

impl From<&'static RuleMetadata> for RuleCatalogItem {
    fn from(rule: &'static RuleMetadata) -> Self {
        Self {
            rule_id: rule.rule_id,
            title: rule.title,
            category: rule.category.label().to_string(),
            severity: rule.default_severity.label().to_string(),
            confidence: rule.default_confidence.label().to_string(),
            lifecycle: rule.lifecycle,
            signal_source: rule.signal_source,
            docs_url: rule.docs_url,
            tags: rule.tags,
            description: rule.description,
            recommendation: rule.recommendation,
            false_positive_notes: rule.false_positive_notes,
        }
    }
}
