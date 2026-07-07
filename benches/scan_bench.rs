//! Scan and changed-review throughput benchmarks.
//!
//! `bench_scan` is the original baseline: a 480-file synthetic repo, kept at
//! its original size so the pre-existing 0.19 budget stays comparable release
//! over release. `bench_full_scan_matrix` and `bench_changed_review_matrix`
//! add the v0.20 benchmark matrix (`docs/engineering/performance-budgets.md`)
//! — medium/large warm full scans, and cold/warm changed review on small and
//! medium synthetic repos.
//!
//! Run with: `cargo bench --bench scan_bench`

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use repopilot::api::scan::{ScanConfig, scan_changed_with_config, scan_path_with_config};
use std::fs;
use std::hint::black_box;
use std::path::Path;
use tempfile::TempDir;

#[path = "../tests/support/synthetic_repo.rs"]
mod synthetic_repo;
use synthetic_repo::{LEGACY_480_FILES_PER_LANG, SyntheticSize};

fn bench_scan(c: &mut Criterion) {
    let repo = synthetic_repo::build_synthetic_repo(LEGACY_480_FILES_PER_LANG);
    let root = repo.path().to_path_buf();
    let config = ScanConfig::default();

    c.bench_function("scan_synthetic_480_files", |b| {
        b.iter(|| {
            let summary = scan_path_with_config(black_box(&root), black_box(&config))
                .expect("scan synthetic repo");
            black_box(summary);
        });
    });
}

fn bench_full_scan_matrix(c: &mut Criterion) {
    let config = ScanConfig::default();

    for (label, size) in [
        ("medium", SyntheticSize::Medium),
        ("large", SyntheticSize::Large),
    ] {
        let repo = synthetic_repo::build_synthetic_repo(size.files_per_lang());
        let root = repo.path().to_path_buf();

        c.bench_function(&format!("scan_full_warm_{label}_synthetic"), |b| {
            b.iter(|| {
                let summary = scan_path_with_config(black_box(&root), black_box(&config))
                    .expect("scan synthetic repo");
                black_box(summary);
            });
        });
    }
}

/// Builds a committed synthetic repo with one uncommitted edit, so every
/// `--changed`-equivalent scan sees exactly one changed file relative to
/// `HEAD` regardless of how many times it runs.
fn build_changed_review_fixture(files_per_lang: usize) -> TempDir {
    let repo = synthetic_repo::build_synthetic_repo(files_per_lang);
    let root = repo.path();
    synthetic_repo::init_git_repo(root);
    synthetic_repo::commit_all(root, "initial");
    let edited = root.join("src/file_0.rs");
    let original = fs::read_to_string(&edited).expect("read edited file");
    fs::write(&edited, format!("{original}\n// benchmark edit\n")).expect("edit file");
    repo
}

fn clear_parsed_cache(root: &Path) {
    let _ = fs::remove_dir_all(root.join(".repopilot/cache"));
}

fn bench_changed_review_matrix(c: &mut Criterion) {
    let config = ScanConfig::default();

    for (label, size) in [
        ("small", SyntheticSize::Small),
        ("medium", SyntheticSize::Medium),
    ] {
        let repo = build_changed_review_fixture(size.files_per_lang());
        let root = repo.path().to_path_buf();

        c.bench_function(&format!("changed_review_cold_{label}_synthetic"), |b| {
            b.iter_batched(
                || clear_parsed_cache(&root),
                |()| {
                    let summary = scan_changed_with_config(
                        black_box(&root),
                        black_box(&config),
                        black_box(None),
                    )
                    .expect("cold changed review");
                    black_box(summary);
                },
                BatchSize::SmallInput,
            );
        });

        // Prime the parsed-facts cache once before measuring the warm path;
        // the edit above never changes, so every subsequent run stays warm.
        scan_changed_with_config(&root, &config, None).expect("warm the parsed cache");
        c.bench_function(&format!("changed_review_warm_{label}_synthetic"), |b| {
            b.iter(|| {
                let summary =
                    scan_changed_with_config(black_box(&root), black_box(&config), black_box(None))
                        .expect("warm changed review");
                black_box(summary);
            });
        });
    }
}

criterion_group! {
    name = benches;
    // A 480-file scan runs ~180ms, so the default 100 samples would take ~19s.
    // 20 samples keeps `cargo bench` fast while still detecting the meaningful
    // (>10%) change the shared parsed-AST work targets.
    config = Criterion::default().sample_size(20);
    targets = bench_scan
}

criterion_group! {
    name = matrix_benches;
    // Medium/large full scans and repo-scale changed review are slower per
    // iteration than the 480-file baseline; 10 samples keeps the full matrix
    // under a couple of minutes while still catching >10% regressions.
    config = Criterion::default().sample_size(10);
    targets = bench_full_scan_matrix, bench_changed_review_matrix
}

criterion_main!(benches, matrix_benches);
