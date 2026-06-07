use crate::review::model::ReviewReport;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReviewSignalGatePolicy {
    None,
    Definitely,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReviewSignalGateResult {
    pub policy: ReviewSignalGatePolicy,
    pub failed_signals: usize,
}

impl ReviewSignalGateResult {
    pub fn evaluate(report: &ReviewReport, policy: ReviewSignalGatePolicy) -> Self {
        let failed_signals = match policy {
            ReviewSignalGatePolicy::None => 0,
            ReviewSignalGatePolicy::Definitely => {
                report.tiered_signals.gate_eligible_definitely_count()
            }
        };
        Self {
            policy,
            failed_signals,
        }
    }

    pub fn passed(&self) -> bool {
        self.failed_signals == 0
    }

    pub fn label(&self) -> &'static str {
        match self.policy {
            ReviewSignalGatePolicy::None => "none",
            ReviewSignalGatePolicy::Definitely => "definitely",
        }
    }
}
