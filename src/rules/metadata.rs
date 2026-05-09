use crate::findings::types::{FindingCategory, Severity};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleMetadata {
    pub rule_id: &'static str,
    pub title: &'static str,
    pub category: FindingCategory,
    pub default_severity: Severity,
    pub docs_url: Option<&'static str>,
    pub description: &'static str,
    pub recommendation: Option<&'static str>,
}
