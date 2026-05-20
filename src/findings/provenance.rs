use crate::rules::{RuleLifecycle, SignalSource};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FindingProvenance {
    pub detector: String,
    pub signal_source: SignalSource,
    pub rule_lifecycle: RuleLifecycle,
    pub analysis_scope: AnalysisScope,
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
        }
    }
}

impl AnalysisScope {
    pub fn for_signal_source(signal_source: SignalSource) -> Self {
        match signal_source {
            SignalSource::ConfigFile
            | SignalSource::DependencyManifest
            | SignalSource::ImportGraph => Self::Repository,
            SignalSource::FrameworkDetector => Self::FrameworkProject,
            SignalSource::GitDiff => Self::GitDiff,
            SignalSource::TextHeuristic | SignalSource::Ast | SignalSource::Mixed => Self::File,
        }
    }
}
