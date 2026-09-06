use serde::Serialize;

use crate::review::model::ReviewReport;

const REMOVED_EXPORT_SIGNAL: &str = "behavioral.removed-export-still-imported";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContractFamily {
    PublicSymbol,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContractChangeKind {
    RemovedExport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
}

pub(crate) fn from_review(report: &ReviewReport) -> Vec<ChangeProofContractDelta> {
    let mut deltas = report
        .tiered_signals
        .definitely
        .iter()
        .filter(|signal| signal.kind == REMOVED_EXPORT_SIGNAL && !signal.suppressed)
        .filter_map(|signal| {
            Some(ChangeProofContractDelta {
                family: ContractFamily::PublicSymbol,
                change: ContractChangeKind::RemovedExport,
                exporter_path: signal.target_path.clone()?,
                consumer_path: signal.path.clone(),
                line_start: signal.line_start,
                line_end: signal.line_end,
                evidence: signal
                    .detail
                    .clone()
                    .unwrap_or_else(|| signal.headline.clone()),
            })
        })
        .collect::<Vec<_>>();
    deltas.sort_by(|left, right| {
        left.exporter_path
            .cmp(&right.exporter_path)
            .then(left.consumer_path.cmp(&right.consumer_path))
            .then(left.line_start.cmp(&right.line_start))
            .then(left.line_end.cmp(&right.line_end))
            .then(left.evidence.cmp(&right.evidence))
    });
    deltas
}
