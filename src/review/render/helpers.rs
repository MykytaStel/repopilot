use crate::baseline::diff::BaselineStatus;
use crate::findings::types::Finding;
use crate::review::diff::ChangedFile;
use crate::review::model::ReviewReport;

pub(super) fn status_for_finding(
    report: &ReviewReport,
    finding: &Finding,
) -> Option<BaselineStatus> {
    report
        .summary
        .findings
        .iter()
        .position(|candidate| candidate == finding)
        .and_then(|index| report.finding_status(index))
        .and_then(|status| status.baseline_status)
}

pub(super) fn render_ranges_suffix(file: &ChangedFile) -> String {
    let ranges = render_ranges(file);
    if ranges == "n/a" {
        String::new()
    } else {
        format!(" ({ranges})")
    }
}

pub(super) fn render_ranges(file: &ChangedFile) -> String {
    if file.ranges.is_empty() {
        return "n/a".to_string();
    }

    file.ranges
        .iter()
        .map(|range| {
            if range.start == range.end {
                range.start.to_string()
            } else {
                format!("{}-{}", range.start, range.end)
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}
