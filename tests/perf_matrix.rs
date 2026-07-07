//! v0.20 benchmark and determinism matrix (PR #270,
//! `docs/roadmap/v0.20.md`, `docs/engineering/performance-budgets.md`).
//!
//! Covers the deterministic half of the matrix as a CI-gated `cargo test`:
//! thread-count determinism on small/medium synthetic repos, and parsed-cache
//! v2 hit/miss/invalidation behavior on the changed-review path (added in
//! #268). The timing/regression half of the matrix is release-only
//! (`scripts/check-scan-performance.js`) since Criterion-scale wall-clock
//! measurements are too noisy for a per-PR CI gate.

#[path = "support/synthetic_repo.rs"]
mod synthetic_repo;

use std::fs;
use std::path::Path;
use std::process::Command;
use synthetic_repo::SyntheticSize;

fn scan_json_with_threads(project: &Path, threads: usize) -> serde_json::Value {
    let _ = fs::remove_dir_all(project.join(".repopilot/cache"));

    let output = Command::new(env!("CARGO_BIN_EXE_repopilot"))
        .args(["scan", ".", "--format", "json"])
        .current_dir(project)
        .env("RAYON_NUM_THREADS", threads.to_string())
        .output()
        .expect("run repopilot scan");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let mut value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("scan JSON output");
    if let Some(object) = value.as_object_mut() {
        object.remove("scan_duration_us");
        object.remove("scan_timings");
    }
    value
}

fn assert_deterministic_across_threads(size: SyntheticSize, label: &str) {
    let repo = synthetic_repo::build_synthetic_repo(size.files_per_lang());
    let root = repo.path();

    let single = scan_json_with_threads(root, 1);
    let dual = scan_json_with_threads(root, 2);
    let quad = scan_json_with_threads(root, 4);

    assert_eq!(
        single, dual,
        "{label} full scan JSON must be deterministic across 1 and 2 worker threads"
    );
    assert_eq!(
        single, quad,
        "{label} full scan JSON must be deterministic across 1 and 4 worker threads"
    );
}

#[test]
fn full_scan_is_deterministic_across_thread_counts_on_small_synthetic_repo() {
    assert_deterministic_across_threads(SyntheticSize::Small, "small synthetic");
}

#[test]
fn full_scan_is_deterministic_across_thread_counts_on_medium_synthetic_repo() {
    assert_deterministic_across_threads(SyntheticSize::Medium, "medium synthetic");
}

fn scan_changed_json(root: &Path, args: &[&str]) -> serde_json::Value {
    let output = Command::new(env!("CARGO_BIN_EXE_repopilot"))
        .args(["scan", ".", "--format", "json"])
        .args(args)
        .current_dir(root)
        .output()
        .expect("run repopilot scan");

    assert!(
        output.status.success(),
        "scan failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice(&output.stdout).expect("scan JSON output")
}

/// Finds a `changed_files[]` entry by path (order is not guaranteed once more
/// than one file changed in a single scan).
fn changed_file_entry<'a>(json: &'a serde_json::Value, path: &str) -> &'a serde_json::Value {
    json["cache_telemetry"]["changed_files"]
        .as_array()
        .expect("changed_files array")
        .iter()
        .find(|entry| entry["path"] == path)
        .unwrap_or_else(|| panic!("no changed_files entry for {path} in {json:#?}"))
}

/// Exercises the three parsed-cache v2 (#268) scenarios from the
/// `performance-budgets.md` cache-behavior matrix on a small git fixture:
/// a changed file, an added file, and a removed file.
#[test]
fn parsed_cache_records_hit_miss_and_invalidation_across_changed_added_and_removed_files() {
    let temp = tempfile::tempdir().expect("temp dir");
    synthetic_repo::init_git_repo(temp.path());
    fs::write(temp.path().join("a.rs"), "pub fn a() {}\n").expect("write a.rs");
    fs::write(temp.path().join("b.rs"), "pub fn b() {}\n").expect("write b.rs");
    fs::write(temp.path().join("c.rs"), "pub fn c() {}\n").expect("write c.rs");
    synthetic_repo::commit_all(temp.path(), "initial");

    // Scenario 1: file changed. The cold pass (no parsed cache on disk yet) is
    // a top-level cache miss for the edited file; the immediate warm re-run
    // (no further edits) reuses it as a hit with zero parsed-cache misses.
    fs::write(temp.path().join("a.rs"), "pub fn a() { let _ = 1 + 1; }\n").expect("edit a.rs");
    let cold_change = scan_changed_json(temp.path(), &["--changed"]);
    assert_eq!(
        changed_file_entry(&cold_change, "a.rs")["cache_status"],
        "miss"
    );
    assert!(
        cold_change["cache_telemetry"]["parsed_cache_misses"]
            .as_u64()
            .is_some_and(|count| count > 0),
        "cold parsed-cache pass must record at least one miss: {cold_change:#?}"
    );

    let warm_change = scan_changed_json(temp.path(), &["--changed"]);
    assert_eq!(warm_change["cache_telemetry"]["hits"], 1);
    assert_eq!(
        changed_file_entry(&warm_change, "a.rs")["cache_status"],
        "hit"
    );
    assert!(
        warm_change["cache_telemetry"]["parsed_cache_hits"]
            .as_u64()
            .is_some_and(|count| count > 0),
        "warm re-run must reuse parsed facts as hits: {warm_change:#?}"
    );
    assert_eq!(warm_change["cache_telemetry"]["parsed_cache_misses"], 0);

    // Scenario 2: file added (untracked, uncommitted). It is a top-level
    // cache miss because it has no prior cache entry; already-cached files
    // are unaffected.
    fs::write(temp.path().join("d.rs"), "pub fn d() {}\n").expect("write d.rs");
    let added = scan_changed_json(temp.path(), &["--changed"]);
    let d_entry = changed_file_entry(&added, "d.rs");
    assert_eq!(d_entry["change_reason"], "untracked");
    assert_eq!(d_entry["cache_status"], "miss");
    assert_eq!(d_entry["cache_reason"], "missing-cache-entry");

    // Scenario 3: file removed. `d.rs` must be committed first so git reports
    // its removal as a tracked deletion (an uncommitted/untracked file simply
    // vanishes with no change status) and it has an existing parsed-cache
    // entry to invalidate.
    synthetic_repo::commit_all(temp.path(), "add d.rs");
    fs::remove_file(temp.path().join("d.rs")).expect("remove d.rs");
    let removed = scan_changed_json(temp.path(), &["--changed"]);
    assert_eq!(
        changed_file_entry(&removed, "d.rs")["change_reason"],
        "deleted"
    );
    assert!(
        removed["cache_telemetry"]["parsed_cache_invalidations"]
            .as_u64()
            .is_some_and(|count| count > 0),
        "removing a file must record a parsed-cache invalidation: {removed:#?}"
    );
}
