use crate::findings::types::Severity;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SupportLevel {
    DetectOnly,
    ImportAware,
    ContextAware,
    RuleAware,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct LanguageProfile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub filenames: Vec<String>,
    pub support: SupportLevel,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FrameworkProfile {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RuntimeProfile {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ParadigmProfile {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RuleApplicability {
    pub rule_id: String,
    #[serde(default)]
    pub minimum_support: Option<SupportLevel>,
    #[serde(default)]
    pub languages: Vec<String>,
    #[serde(default)]
    pub frameworks: Vec<String>,
    #[serde(default)]
    pub runtimes: Vec<String>,
    #[serde(default)]
    pub paradigms: Vec<String>,
    #[serde(default)]
    pub suppress_low_signal: bool,
    #[serde(default)]
    pub suppress_generated: bool,
    #[serde(default)]
    pub suppress_config: bool,
    #[serde(default)]
    pub overrides: Vec<RuleOverride>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RuleOverride {
    #[serde(default)]
    pub signal: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub framework: Option<String>,
    #[serde(default)]
    pub runtime: Option<String>,
    #[serde(default)]
    pub paradigm: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    pub action: RuleDecisionAction,
    #[serde(default)]
    pub severity: Option<Severity>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct KnowledgePack {
    #[serde(default)]
    pub languages: Vec<LanguageProfile>,
    #[serde(default)]
    pub frameworks: Vec<FrameworkProfile>,
    #[serde(default)]
    pub runtimes: Vec<RuntimeProfile>,
    #[serde(default)]
    pub paradigms: Vec<ParadigmProfile>,
    #[serde(default, rename = "rules")]
    pub rule_applicability: Vec<RuleApplicability>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeBase {
    pub languages: Vec<LanguageProfile>,
    pub frameworks: Vec<FrameworkProfile>,
    pub runtimes: Vec<RuntimeProfile>,
    pub paradigms: Vec<ParadigmProfile>,
    pub rule_applicability: Vec<RuleApplicability>,
}

impl From<KnowledgePack> for KnowledgeBase {
    fn from(pack: KnowledgePack) -> Self {
        Self {
            languages: pack.languages,
            frameworks: pack.frameworks,
            runtimes: pack.runtimes,
            paradigms: pack.paradigms,
            rule_applicability: pack.rule_applicability,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuleDecisionAction {
    Apply,
    Suppress,
    Downgrade,
    Upgrade,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleDecision {
    pub action: RuleDecisionAction,
    pub severity: Severity,
    pub reason: Option<String>,
}

impl RuleDecision {
    pub fn apply(severity: Severity) -> Self {
        Self {
            action: RuleDecisionAction::Apply,
            severity,
            reason: None,
        }
    }

    pub fn suppress(reason: impl Into<String>) -> Self {
        Self {
            action: RuleDecisionAction::Suppress,
            severity: Severity::Info,
            reason: Some(reason.into()),
        }
    }

    pub fn is_suppressed(&self) -> bool {
        self.action == RuleDecisionAction::Suppress
    }
}

#[derive(Debug, Clone)]
pub struct RuleMatchContext<'a> {
    pub rule_id: &'a str,
    pub languages: &'a [&'a str],
    pub frameworks: &'a [&'a str],
    pub roles: &'a [&'a str],
    pub paradigms: &'a [&'a str],
    pub runtimes: &'a [&'a str],
    pub is_test: bool,
    pub is_low_signal: bool,
    pub signal: Option<&'a str>,
    pub base_severity: Severity,
}
