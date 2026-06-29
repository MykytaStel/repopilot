use crate::findings::severity::Severity;
use crate::rules::{RuleLifecycle, SignalSource};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FindingProvenance {
    pub detector: String,
    pub signal_source: SignalSource,
    pub rule_lifecycle: RuleLifecycle,
    pub analysis_scope: AnalysisScope,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub knowledge_decision: Option<KnowledgeDecisionProvenance>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KnowledgeDecisionProvenance {
    pub base_severity: Severity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signal: Option<String>,
    pub action: KnowledgeDecisionAction,
    pub decided_severity: Severity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum KnowledgeDecisionAction {
    Apply,
    Suppress,
    Downgrade,
    Upgrade,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AnalysisScope {
    #[default]
    File,
    Repository,
    Workspace,
    GitDiff,
    FrameworkProject,
}

impl Default for FindingProvenance {
    fn default() -> Self {
        Self {
            detector: "unknown".to_string(),
            signal_source: SignalSource::Mixed,
            rule_lifecycle: RuleLifecycle::Preview,
            analysis_scope: AnalysisScope::File,
            knowledge_decision: None,
        }
    }
}

impl FindingProvenance {
    /// Registry metadata may be missing even after a runner has stamped scope.
    pub fn has_default_metadata(&self) -> bool {
        self.detector == "unknown"
            && self.signal_source == SignalSource::Mixed
            && self.rule_lifecycle == RuleLifecycle::Preview
    }
}
