use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum RuleLifecycle {
    Experimental,
    #[default]
    Preview,
    Stable,
    Deprecated,
}

impl RuleLifecycle {
    pub fn label(self) -> &'static str {
        match self {
            Self::Experimental => "experimental",
            Self::Preview => "preview",
            Self::Stable => "stable",
            Self::Deprecated => "deprecated",
        }
    }
}
