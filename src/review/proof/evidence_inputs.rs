//! Which provenance inputs a review actually recorded. Only inputs RepoPilot
//! did not capture are reported as unavailable.

use crate::review::model::ReviewReport;
use crate::scan::types::ScanMode;

pub(super) fn unavailable_inputs(report: &ReviewReport) -> Vec<String> {
    let revisions = &report.revisions;
    let mut inputs = vec!["scanner configuration".to_string(), "toolchain".to_string()];
    if report.analysis_revision.is_none() {
        inputs.push("current revision".to_string());
    }
    let head_recorded = revisions.head_commit.is_some()
        || (revisions.head_is_working_tree && report.analysis_revision.is_some());
    if !head_recorded {
        inputs.push("head revision".to_string());
    }
    if report.summary.mode == ScanMode::Changed && revisions.base_commit.is_none() {
        inputs.push("base revision".to_string());
    }
    inputs.sort();
    inputs
}

/// Short, human-readable revision summary, e.g. `base 1f17e2c7, head 6a49e4bb`.
pub(super) fn revision_summary(base: Option<&str>, head: Option<&str>) -> Option<String> {
    let parts = [("base", base), ("head", head)]
        .into_iter()
        .filter_map(|(label, commit)| commit.map(|commit| format!("{label} {}", short(commit))))
        .collect::<Vec<_>>();
    (!parts.is_empty()).then(|| parts.join(", "))
}

fn short(commit: &str) -> &str {
    commit.get(..8).unwrap_or(commit)
}
