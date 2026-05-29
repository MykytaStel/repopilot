use repopilot::scan::config::ScanConfig;
use repopilot::scan::scanner::{collect_scan_facts, scan_path_with_config};
use std::fs;
use tempfile::tempdir;

/// Verifies that the parallel scan pipeline produces the same counts as the
/// sequential collect path when run on the same directory.
#[test]
fn parallel_scan_matches_sequential_file_counts() {
    let temp = tempdir().unwrap();

    for i in 0..20 {
        fs::write(
            temp.path().join(format!("file_{i}.rs")),
            format!("fn func_{i}() {{}}\n"),
        )
        .unwrap();
    }
    fs::write(temp.path().join("script.ts"), "export const x = 1;\n").unwrap();

    let sequential = collect_scan_facts(temp.path()).unwrap();
    let parallel = scan_path_with_config(temp.path(), &ScanConfig::default()).unwrap();

    assert_eq!(
        sequential.files_analyzed, parallel.metrics.files_analyzed,
        "file count must match between sequential and parallel paths"
    );
    assert_eq!(
        sequential.non_empty_lines, parallel.metrics.non_empty_lines,
        "LOC count must match"
    );
    assert_eq!(
        sequential.languages.len(),
        parallel.metrics.languages.len(),
        "detected language count must match"
    );
}

/// Verifies that scan_duration_us is recorded. Even a trivial scan takes at least
/// a few microseconds, so the field must be non-zero.
#[test]
fn scan_duration_is_recorded() {
    let temp = tempdir().unwrap();
    // Write enough files that the scan takes a measurable number of microseconds.
    for i in 0..10 {
        fs::write(
            temp.path().join(format!("file_{i}.rs")),
            format!("fn func_{i}() {{}}\n"),
        )
        .unwrap();
    }

    let summary = scan_path_with_config(temp.path(), &ScanConfig::default()).unwrap();

    assert!(
        summary.scan_duration_us > 0,
        "scan_duration_us should be non-zero"
    );
}

#[test]
fn scan_timings_expose_pipeline_stage_breakdown() {
    let temp = tempdir().unwrap();
    fs::write(temp.path().join("file.rs"), "fn main() {}\n").unwrap();

    let summary = scan_path_with_config(temp.path(), &ScanConfig::default()).unwrap();
    let timings = summary
        .scan_timings
        .as_ref()
        .expect("full scan should expose engine timings");

    assert_eq!(
        timings.file_scan_us,
        timings.discovery_us + timings.file_analysis_us,
        "legacy file_scan_us should remain the discovery + file analysis aggregate"
    );
    assert!(
        timings.accounted_engine_us() >= timings.file_scan_us,
        "accounted timing should include file scan plus later pipeline stages"
    );
    assert!(
        timings.accounted_engine_us()
            >= timings
                .file_scan_us
                .saturating_add(timings.contract_validation_us),
        "accounted timing should include contract validation"
    );
}

/// Verifies that parallel scan handles an empty directory cleanly without panicking.
#[test]
fn parallel_scan_empty_directory() {
    let temp = tempdir().unwrap();
    let summary = scan_path_with_config(temp.path(), &ScanConfig::default()).unwrap();

    assert_eq!(summary.metrics.files_analyzed, 0);
    assert_eq!(summary.metrics.non_empty_lines, 0);
    // Project-level audits (e.g. missing-test-folder) may still produce findings
    // for an empty directory — that is expected behaviour.
}
