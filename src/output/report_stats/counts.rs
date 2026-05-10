use crate::findings::types::Severity;
use crate::output::report_stats::NamedCount;
use crate::output::report_text::category_label_rank;
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct CountWithSeverity {
    count: usize,
    severity: Option<Severity>,
}

pub(super) fn increment(count: &mut CountWithSeverity, severity: Severity) {
    count.count += 1;
    count.severity = Some(
        count
            .severity
            .map_or(severity, |current| current.max(severity)),
    );
}

pub(super) fn category_counts_from_map(
    map: BTreeMap<&'static str, CountWithSeverity>,
) -> Vec<NamedCount> {
    let mut counts = map
        .into_iter()
        .map(|(label, count)| NamedCount {
            label: label.to_string(),
            count: count.count,
            severity: count.severity,
        })
        .collect::<Vec<_>>();
    counts.sort_by(|left, right| {
        category_label_rank(&left.label)
            .cmp(&category_label_rank(&right.label))
            .then_with(|| left.label.cmp(&right.label))
    });
    counts
}

pub(super) fn top_counts_from_map(
    map: BTreeMap<String, CountWithSeverity>,
    limit: usize,
) -> Vec<NamedCount> {
    let mut counts = map
        .into_iter()
        .map(|(label, count)| NamedCount {
            label,
            count: count.count,
            severity: count.severity,
        })
        .collect::<Vec<_>>();
    counts.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| right.severity.cmp(&left.severity))
            .then_with(|| left.label.cmp(&right.label))
    });
    counts.truncate(limit);
    counts
}
