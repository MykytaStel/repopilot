use repopilot::scan::types::ScanSummary;
use std::time::Duration;

pub(super) fn print_verbose_scan_timing(
    summary: &ScanSummary,
    scan_elapsed: Duration,
    render_elapsed: Duration,
) {
    let internal_us = summary.scan_duration_us;
    let total_ms = scan_elapsed.as_millis();
    let render_ms = render_elapsed.as_millis();

    eprintln!(
        "\n[verbose] Scan: {total_ms}ms (engine: {:.0}ms) · Render: {render_ms}ms",
        internal_us as f64 / 1000.0
    );
}

pub(super) fn print_timing_breakdown(summary: &ScanSummary) {
    if let Some(timings) = &summary.scan_timings {
        eprintln!(
            "\n[timing] File scan: {}ms · Framework detection: {}ms · Post-scan audits: {}ms · Engine total: {}ms",
            timings.file_scan_us / 1000,
            timings.framework_detection_us / 1000,
            timings.post_scan_audits_us / 1000,
            timings.accounted_engine_us() / 1000,
        );
        eprintln!(
            "[timing] Pipeline: discovery {}ms · file analysis {}ms · enrichment {}ms · risk scoring {}ms · contract validation {}ms · report finalization {}ms",
            timings.discovery_us / 1000,
            timings.file_analysis_us / 1000,
            timings.enrichment_us / 1000,
            timings.risk_scoring_us / 1000,
            timings.contract_validation_us / 1000,
            timings.report_finalization_us / 1000,
        );
    }

    if let Some(cache) = &summary.cache_telemetry {
        let estimated_saved = cache
            .timings
            .estimated_time_saved_us
            .map(|value| format!("{}ms", value / 1000))
            .unwrap_or_else(|| "n/a".to_string());
        eprintln!(
            "[timing] Cache: load {}ms · hash {}ms · lookup {}ms · hit reuse {}ms · miss scan {}ms · write {}ms · estimated saved {}",
            cache.timings.load_us / 1000,
            cache.timings.file_hash_us / 1000,
            cache.timings.lookup_us / 1000,
            cache.timings.hit_reuse_us / 1000,
            cache.timings.miss_scan_us / 1000,
            cache.timings.write_us / 1000,
            estimated_saved,
        );
    }
}
