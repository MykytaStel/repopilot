use crate::findings::quality::{SignalQualitySummary, summarize_signal_quality};
use crate::findings::types::Finding;

pub fn build_signal_quality_summary(findings: &[Finding]) -> SignalQualitySummary {
    summarize_signal_quality(findings)
}
