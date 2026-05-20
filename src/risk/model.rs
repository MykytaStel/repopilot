use crate::baseline::diff::BaselineStatus;
use serde::{Deserialize, Serialize};

pub const FORMULA_VERSION: &str = "risk-v3";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RiskFormula {
    pub version: &'static str,
    pub severity_critical: u8,
    pub severity_high: u8,
    pub severity_medium: u8,
    pub severity_low: u8,
    pub severity_info: u8,
    pub confidence_high_percent: u16,
    pub confidence_medium_percent: u16,
    pub confidence_low_percent: u16,
    pub security_category_weight: i16,
    pub review_in_diff_weight: i16,
    pub workspace_hotspot_weight: i16,
    pub graph_hub_weight: i16,
    pub graph_dependency_weight: i16,
    pub blast_radius_weight: i16,
    pub cluster_small_weight: i16,
    pub cluster_medium_weight: i16,
    pub cluster_large_weight: i16,
}

impl RiskFormula {
    pub const CURRENT: Self = Self {
        version: FORMULA_VERSION,
        severity_critical: 95,
        severity_high: 75,
        severity_medium: 45,
        severity_low: 20,
        severity_info: 5,
        confidence_high_percent: 110,
        confidence_medium_percent: 100,
        confidence_low_percent: 80,
        security_category_weight: 12,
        review_in_diff_weight: 12,
        workspace_hotspot_weight: 5,
        graph_hub_weight: 8,
        graph_dependency_weight: 5,
        blast_radius_weight: 6,
        cluster_small_weight: 3,
        cluster_medium_weight: 5,
        cluster_large_weight: 7,
    };
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RiskAssessment {
    pub score: u8,
    pub priority: RiskPriority,
    pub signals: Vec<RiskSignal>,
    pub formula_version: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RiskPriority {
    P0,
    P1,
    P2,
    P3,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RiskSignal {
    pub id: String,
    pub label: String,
    pub weight: i16,
    pub reason: String,
    #[serde(default)]
    pub source: RiskSignalSource,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RiskSignalSource {
    #[default]
    Severity,
    Confidence,
    Category,
    Graph,
    ReviewDiff,
    Workspace,
    Baseline,
    Cluster,
    Visibility,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RiskInputs {
    pub baseline_status: Option<BaselineStatus>,
    pub in_diff: bool,
    pub workspace_hotspot: bool,
    pub graph_impact: Option<GraphImpact>,
    pub blast_radius: bool,
    pub cluster_size: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphImpact {
    Hub,
    Dependency,
}

impl Default for RiskAssessment {
    fn default() -> Self {
        Self {
            score: 0,
            priority: RiskPriority::P3,
            signals: Vec::new(),
            formula_version: FORMULA_VERSION.to_string(),
        }
    }
}

impl RiskAssessment {
    pub(super) fn new(score: u8, signals: Vec<RiskSignal>) -> Self {
        Self {
            score,
            priority: priority_for_score(score),
            signals,
            formula_version: FORMULA_VERSION.to_string(),
        }
    }
}

pub fn priority_for_score(score: u8) -> RiskPriority {
    match score {
        90..=100 => RiskPriority::P0,
        70..=89 => RiskPriority::P1,
        40..=69 => RiskPriority::P2,
        _ => RiskPriority::P3,
    }
}

impl RiskPriority {
    pub fn label(self) -> &'static str {
        match self {
            Self::P0 => "P0",
            Self::P1 => "P1",
            Self::P2 => "P2",
            Self::P3 => "P3",
        }
    }

    pub fn rank(self) -> u8 {
        match self {
            Self::P0 => 0,
            Self::P1 => 1,
            Self::P2 => 2,
            Self::P3 => 3,
        }
    }

    pub fn is_at_least(self, threshold: Self) -> bool {
        self.rank() <= threshold.rank()
    }
}

pub(super) fn push_adjustment(
    score: &mut i16,
    signals: &mut Vec<RiskSignal>,
    id: &str,
    label: &str,
    weight: i16,
    reason: &str,
) {
    push_signal(score, signals, signal(id, label, weight, reason));
}

pub(super) fn push_signal(score: &mut i16, signals: &mut Vec<RiskSignal>, signal: RiskSignal) {
    *score += signal.weight;
    signals.push(signal);
}

pub(super) fn signal(id: &str, label: &str, weight: i16, reason: &str) -> RiskSignal {
    RiskSignal {
        id: id.to_string(),
        label: label.to_string(),
        weight,
        reason: reason.to_string(),
        source: source_for_signal_id(id),
    }
}

fn source_for_signal_id(id: &str) -> RiskSignalSource {
    if id.starts_with("severity.") {
        RiskSignalSource::Severity
    } else if id.starts_with("confidence.") {
        RiskSignalSource::Confidence
    } else if id.starts_with("category.") || id.starts_with("knowledge.") {
        RiskSignalSource::Category
    } else if id.starts_with("graph.") {
        RiskSignalSource::Graph
    } else if id.starts_with("review.") {
        RiskSignalSource::ReviewDiff
    } else if id.starts_with("workspace.") {
        RiskSignalSource::Workspace
    } else if id.starts_with("baseline.") {
        RiskSignalSource::Baseline
    } else if id.starts_with("cluster.") {
        RiskSignalSource::Cluster
    } else {
        RiskSignalSource::Visibility
    }
}

pub(super) fn clamp_score(score: i16) -> u8 {
    score.clamp(0, 100) as u8
}
