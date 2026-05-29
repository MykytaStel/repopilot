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
    /// The default runtime calibration formula weights.
    pub const CURRENT: Self = Self {
        version: FORMULA_VERSION,
        // Base severity scores (0-100 range)
        // Critical findings represent immediate action items (e.g. committed credentials).
        severity_critical: 95,
        // High findings point to high-impact architectural risks or severe issues.
        severity_high: 75,
        // Medium findings capture common code quality and minor design flaws.
        severity_medium: 45,
        // Low findings represent minor warnings with low potential runtime impact.
        severity_low: 20,
        // Info findings are informational and carry negligible base risk.
        severity_info: 5,

        // Confidence multipliers (percentage multipliers applied to base severity)
        // Boost score slightly for high-confidence static findings.
        confidence_high_percent: 110,
        // Keep standard score for medium-confidence findings.
        confidence_medium_percent: 100,
        // Penalize/dampen score for lower confidence heuristics.
        confidence_low_percent: 80,

        // Category & Context weights (added directly to final score)
        // Security findings carry a high category penalty due to potential breach risks.
        security_category_weight: 12,
        // Touch findings that appear directly in git diffs get higher visibility priority.
        review_in_diff_weight: 12,
        // Hotspot files have higher density of findings/churn, increasing risk.
        workspace_hotspot_weight: 5,
        // Graph hubs are heavily imported, so findings inside them spread risk downstream.
        graph_hub_weight: 8,
        // Shared dependencies carry moderate graph propagation risk.
        graph_dependency_weight: 5,
        // A wide downstream blast radius adds weight to the risk.
        blast_radius_weight: 6,

        // Cluster weights (penalties for repeated/correlated findings in the same scope)
        // Small cluster penalty (2-3 correlated findings).
        cluster_small_weight: 3,
        // Medium cluster penalty (4-7 correlated findings).
        cluster_medium_weight: 5,
        // Large cluster penalty (8+ correlated findings).
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
