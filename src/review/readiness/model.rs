use crate::history::RiskDelta;
use crate::review::ImpactPaths;
use crate::review::ownership::OwnershipSummary;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ReadinessVerdict {
    Ready,
    Review,
    Blocked,
}

impl ReadinessVerdict {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Review => "review",
            Self::Blocked => "blocked",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReadinessReasonCode {
    AnalysisError,
    FindingGateFailed,
    ReviewSignalGateFailed,
    PriorityP0,
    PriorityP1,
    DefinitelySensitive,
    MaybeSensitive,
    BoundaryMissingTest,
    VisibleFinding,
    UnownedSurface,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReadinessReason {
    pub code: ReadinessReasonCode,
    pub count: usize,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MergeReadinessRecord {
    pub verdict: ReadinessVerdict,
    pub reasons: Vec<ReadinessReason>,
    pub impact: ImpactPaths,
    pub ownership: OwnershipSummary,
    pub verification_steps: Vec<String>,
    pub limitations: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_delta: Option<RiskDelta>,
}
