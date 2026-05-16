use crate::findings::types::Finding;
use serde::{Deserialize, Serialize};

use super::model::RiskPriority;

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct RiskPriorityCounts {
    pub p0: usize,
    pub p1: usize,
    pub p2: usize,
    pub p3: usize,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RiskSummary {
    pub total: usize,
    pub counts: RiskPriorityCounts,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highest_priority: Option<RiskPriority>,
    pub average_score: u8,
}

impl RiskSummary {
    pub fn from_findings(findings: &[Finding]) -> Self {
        if findings.is_empty() {
            return Self::default();
        }

        let mut counts = RiskPriorityCounts::default();
        let mut highest_priority = RiskPriority::P3;
        let mut score_sum = 0usize;

        for finding in findings {
            counts.increment(finding.risk.priority);
            if priority_rank(finding.risk.priority) < priority_rank(highest_priority) {
                highest_priority = finding.risk.priority;
            }
            score_sum += finding.risk.score as usize;
        }

        let average_score = ((score_sum as f64) / (findings.len() as f64)).round() as u8;

        Self {
            total: findings.len(),
            counts,
            highest_priority: Some(highest_priority),
            average_score,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total == 0
    }
}

impl RiskPriorityCounts {
    pub fn count(&self, priority: RiskPriority) -> usize {
        match priority {
            RiskPriority::P0 => self.p0,
            RiskPriority::P1 => self.p1,
            RiskPriority::P2 => self.p2,
            RiskPriority::P3 => self.p3,
        }
    }

    fn increment(&mut self, priority: RiskPriority) {
        match priority {
            RiskPriority::P0 => self.p0 += 1,
            RiskPriority::P1 => self.p1 += 1,
            RiskPriority::P2 => self.p2 += 1,
            RiskPriority::P3 => self.p3 += 1,
        }
    }
}

fn priority_rank(priority: RiskPriority) -> u8 {
    match priority {
        RiskPriority::P0 => 0,
        RiskPriority::P1 => 1,
        RiskPriority::P2 => 2,
        RiskPriority::P3 => 3,
    }
}
