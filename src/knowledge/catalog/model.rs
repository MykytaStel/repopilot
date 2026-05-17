use crate::knowledge::model::{LanguageProfile, SupportLevel};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnowledgeCatalogSection {
    All,
    Languages,
    Frameworks,
    Runtimes,
    Paradigms,
    Rules,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct KnowledgeCatalogReport {
    pub summary: KnowledgeCatalogSummary,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub languages: Vec<KnowledgeLanguageEntry>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub frameworks: Vec<KnowledgeNamedEntry>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub runtimes: Vec<KnowledgeNamedEntry>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub paradigms: Vec<KnowledgeNamedEntry>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<KnowledgeRuleEntry>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct KnowledgeCatalogSummary {
    pub languages: usize,
    pub frameworks: usize,
    pub runtimes: usize,
    pub paradigms: usize,
    pub rules: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct KnowledgeLanguageEntry {
    pub id: String,
    pub name: String,
    pub support: String,
    pub extensions: Vec<String>,
    pub filenames: Vec<String>,
    pub aliases: Vec<String>,
}

impl KnowledgeLanguageEntry {
    pub(super) fn from_profile(language: &LanguageProfile) -> Self {
        Self {
            id: language.id.clone(),
            name: language.name.clone(),
            support: support_level_label(language.support).to_string(),
            extensions: language.extensions.clone(),
            filenames: language.filenames.clone(),
            aliases: language.aliases.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct KnowledgeNamedEntry {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct KnowledgeRuleEntry {
    pub rule_id: String,
    pub minimum_support: Option<String>,
    pub languages: Vec<String>,
    pub frameworks: Vec<String>,
    pub runtimes: Vec<String>,
    pub paradigms: Vec<String>,
    pub suppress_low_signal: bool,
    pub suppress_generated: bool,
    pub suppress_config: bool,
    pub overrides: usize,
}

pub(super) fn support_level_label(level: SupportLevel) -> &'static str {
    match level {
        SupportLevel::DetectOnly => "detect-only",
        SupportLevel::ImportAware => "import-aware",
        SupportLevel::ContextAware => "context-aware",
        SupportLevel::RuleAware => "rule-aware",
    }
}
