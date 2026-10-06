use crate::review::contract::{
    ChangeProofContractDelta, ContractChangeKind, ContractConfidence, ContractFamily,
};
use crate::review::model::ReviewReport;

mod delivery;
mod dependency;
mod runtime;
mod security;

const REMOVED_EXPORT_SIGNAL: &str = "behavioral.removed-export-still-imported";
const RUST_ARITY_SIGNAL: &str = "behavioral.rust-public-function-arity-changed";

pub(crate) fn from_review(report: &ReviewReport) -> Vec<ChangeProofContractDelta> {
    let mut deltas = report
        .tiered_signals
        .definitely
        .iter()
        .filter(|signal| {
            matches!(
                signal.kind.as_str(),
                REMOVED_EXPORT_SIGNAL | RUST_ARITY_SIGNAL
            ) && !signal.suppressed
        })
        .filter_map(|signal| {
            let change = match signal.kind.as_str() {
                REMOVED_EXPORT_SIGNAL => ContractChangeKind::RemovedExport,
                RUST_ARITY_SIGNAL => ContractChangeKind::FunctionArityChanged,
                _ => return None,
            };
            Some(ChangeProofContractDelta {
                family: ContractFamily::PublicSymbol,
                change,
                exporter_path: signal.target_path.clone()?,
                consumer_path: signal.path.clone(),
                line_start: signal.line_start,
                line_end: signal.line_end,
                evidence: signal
                    .detail
                    .clone()
                    .unwrap_or_else(|| signal.headline.clone()),
                confidence: Some(ContractConfidence::High),
            })
        })
        .collect::<Vec<_>>();
    deltas.extend(dependency::dependency_deltas(&report.changed_files));
    deltas.extend(delivery::delivery_deltas(&report.changed_files));
    deltas.extend(runtime::runtime_deltas_in_repo(
        &report.repo_root,
        &report.changed_files,
    ));
    deltas.extend(security::security_deltas(report));
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
