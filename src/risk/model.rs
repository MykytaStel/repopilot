use crate::baseline::diff::BaselineStatus;
use serde::{Deserialize, Serialize};

pub const FORMULA_VERSION: &str = "risk-v2";

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
    }
}

pub(super) fn clamp_score(score: i16) -> u8 {
    score.clamp(0, 100) as u8
}
