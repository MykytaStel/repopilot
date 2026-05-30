//! Baseline scan-throughput benchmark.
//!
//! Builds a synthetic multi-language repository in a temp directory once, then
//! measures `scan_path_with_config` wall time. This establishes a performance
//! baseline before the shared parsed-AST work (PR 5–7) so the dedup win is
//! provable, and guards against scan regressions afterwards.
//!
//! Run with: `cargo bench --bench scan_bench`

use criterion::{Criterion, criterion_group, criterion_main};
use repopilot::api::scan::{ScanConfig, scan_path_with_config};
use std::fs;
use std::hint::black_box;
use std::path::Path;
use tempfile::TempDir;

/// Files generated per language. 120 × 4 languages = 480 source files, enough
/// to exercise the parallel file pipeline, the AST audits, and the import graph
/// at a realistic small-repo scale without making the benchmark slow.
const FILES_PER_LANG: usize = 120;

const RUST_TEMPLATE: &str = r#"use crate::file_PREVNUM;

pub fn compute_IDXNUM(input: i64) -> i64 {
    let mut total = 0_i64;
    for outer in 0..input {
        if outer % 2 == 0 {
            for inner in 0..outer {
                if inner % 3 == 0 {
                    total += outer * inner;
                } else {
                    total -= inner;
                }
            }
        }
    }
    total
}
"#;

const TS_TEMPLATE: &str = r#"import { compute } from "./file_PREVNUM";

export function run_IDXNUM(input: number): number {
    let total = 0;
    for (let outer = 0; outer < input; outer++) {
        if (outer % 2 === 0) {
            for (let inner = 0; inner < outer; inner++) {
                if (inner % 3 === 0) {
                    total += outer * inner;
                } else {
                    total -= inner;
                }
            }
        }
    }
    return total + compute;
}
"#;

const PY_TEMPLATE: &str = r#"from .file_PREVNUM import compute


def run_IDXNUM(value):
    total = 0
    for outer in range(value):
        if outer % 2 == 0:
            for inner in range(outer):
                if inner % 3 == 0:
                    total += outer * inner
                else:
                    total -= inner
    return total + compute
"#;

const GO_TEMPLATE: &str = r#"package gopkg

import "fmt"

func RunIDXNUM(value int) int {
    total := 0
    for outer := 0; outer < value; outer++ {
        if outer%2 == 0 {
            for inner := 0; inner < outer; inner++ {
                if inner%3 == 0 {
                    total += outer * inner
                } else {
                    total -= inner
                }
            }
        }
    }
    fmt.Sprintln(total)
    return total
}
"#;

fn render(template: &str, index: usize) -> String {
    let prev = index.saturating_sub(1);
    template
        .replace("PREVNUM", &prev.to_string())
        .replace("IDXNUM", &index.to_string())
}

fn write_file(root: &Path, subdir: &str, name: String, contents: String) {
    let dir = root.join(subdir);
    fs::create_dir_all(&dir).expect("create fixture subdir");
    fs::write(dir.join(name), contents).expect("write fixture file");
}

fn build_synthetic_repo() -> TempDir {
    let dir = tempfile::tempdir().expect("create temp repo");
    let root = dir.path();

    // A go.mod lets the Go resolver attempt module-relative resolution.
    fs::write(root.join("go.mod"), "module benchmod\n\ngo 1.22\n").expect("write go.mod");

    for index in 0..FILES_PER_LANG {
        write_file(
            root,
            "src",
            format!("file_{index}.rs"),
            render(RUST_TEMPLATE, index),
        );
        write_file(
            root,
            "web",
            format!("file_{index}.ts"),
            render(TS_TEMPLATE, index),
        );
        write_file(
            root,
            "py",
            format!("file_{index}.py"),
            render(PY_TEMPLATE, index),
        );
        write_file(
            root,
            "gopkg",
            format!("file_{index}.go"),
            render(GO_TEMPLATE, index),
        );
    }

    dir
}

fn bench_scan(c: &mut Criterion) {
    let repo = build_synthetic_repo();
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

criterion_group! {
    name = benches;
    // A 480-file scan runs ~180ms, so the default 100 samples would take ~19s.
    // 20 samples keeps `cargo bench` fast while still detecting the meaningful
    // (>10%) change the shared parsed-AST work targets.
    config = Criterion::default().sample_size(20);
    targets = bench_scan
}
criterion_main!(benches);
