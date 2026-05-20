use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum SignalSource {
    TextHeuristic,
    Ast,
    ConfigFile,
    DependencyManifest,
    ImportGraph,
    FrameworkDetector,
    GitDiff,
    #[default]
    Mixed,
}

impl SignalSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::TextHeuristic => "text-heuristic",
            Self::Ast => "ast",
            Self::ConfigFile => "config-file",
            Self::DependencyManifest => "dependency-manifest",
            Self::ImportGraph => "import-graph",
            Self::FrameworkDetector => "framework-detector",
            Self::GitDiff => "git-diff",
            Self::Mixed => "mixed",
        }
    }
}
