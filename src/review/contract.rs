use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContractFamily {
    PublicSymbol,
    Dependency,
    Delivery,
    RuntimeConfiguration,
    SecurityBoundary,
    TestCoverage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContractChangeKind {
    RemovedExport,
    Added,
    Removed,
    Upgraded,
    Downgraded,
    SourceChanged,
    FeatureChanged,
    AliasChanged,
    MetadataOnly,
    TriggerChanged,
    PermissionChanged,
    SecretUseChanged,
    ActionReferenceChanged,
    ArtifactChanged,
    DeploymentChanged,
    Introduced,
    Renamed,
    Changed,
    Unknown,
    BoundaryChanged,
    EntryPointImpacted,
    TestChanged,
    TestMissing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeProofContractDelta {
    pub family: ContractFamily,
    #[serde(rename = "change")]
    pub change: ContractChangeKind,
    pub exporter_path: String,
    pub consumer_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_start: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_end: Option<usize>,
    pub evidence: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<ContractConfidence>,
}

impl ChangeProofContractDelta {
    pub(crate) fn is_broken(&self) -> bool {
        self.family == ContractFamily::PublicSymbol
            && self.change == ContractChangeKind::RemovedExport
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContractConfidence {
    High,
    Limited,
}
