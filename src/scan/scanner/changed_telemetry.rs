use crate::review::diff::ChangeStatus;
use crate::scan::types::{ChangedFileCacheTelemetry, ChangedFileReasonSummary, ScanCacheTelemetry};
use std::collections::BTreeMap;
use std::path::Path;

pub(super) fn record_skipped_cache_file(
    telemetry: &mut ScanCacheTelemetry,
    path: &Path,
    change_reason: &str,
    cache_reason: &str,
) {
    telemetry.skipped += 1;
    telemetry.changed_files.push(ChangedFileCacheTelemetry {
        path: path.to_path_buf(),
        change_reason: change_reason.to_string(),
        cache_status: "skipped".to_string(),
        cache_reason: cache_reason.to_string(),
    });
}

pub(super) fn finalize_cache_telemetry(
    telemetry: &mut ScanCacheTelemetry,
    changed_file_reasons: BTreeMap<String, usize>,
) {
    let cached_total = telemetry.hits.saturating_add(telemetry.misses);
    let hit_rate = telemetry
        .hits
        .saturating_mul(100)
        .checked_div(cached_total)
        .unwrap_or(0);
    telemetry.hit_rate_percent = hit_rate.min(100) as u8;
    telemetry.changed_file_reasons = changed_file_reasons
        .into_iter()
        .map(|(reason, count)| ChangedFileReasonSummary { reason, count })
        .collect();

    let average_miss_us = telemetry
        .timings
        .miss_scan_us
        .checked_div(telemetry.misses as u64);
    let average_hit_reuse_us = telemetry
        .timings
        .hit_reuse_us
        .checked_div(telemetry.hits as u64);
    telemetry.timings.estimated_time_saved_us =
        average_miss_us
            .zip(average_hit_reuse_us)
            .map(|(average_miss_us, average_hit_reuse_us)| {
                average_miss_us
                    .saturating_sub(average_hit_reuse_us)
                    .saturating_mul(telemetry.hits as u64)
            });
}

pub(super) fn change_status_label(status: ChangeStatus) -> &'static str {
    match status {
        ChangeStatus::Added => "added",
        ChangeStatus::Modified => "modified",
        ChangeStatus::Deleted => "deleted",
        ChangeStatus::Renamed => "renamed",
        ChangeStatus::Untracked => "untracked",
    }
}
