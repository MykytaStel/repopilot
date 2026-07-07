//! Shared synthetic multi-language repository generator for the v0.20
//! benchmark and determinism matrix. Included via `#[path = ...]` from both
//! `benches/scan_bench.rs` and `tests/perf_matrix.rs` so the fixture shape is
//! defined once.
#![allow(dead_code)]

use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// Named sizes for the v0.20 benchmark matrix
/// (`docs/engineering/performance-budgets.md`). Distinct from
/// [`LEGACY_480_FILES_PER_LANG`], which preserves the pre-existing 0.19
/// 480-file benchmark identity.
#[derive(Clone, Copy, Debug)]
pub enum SyntheticSize {
    /// ~60 files: fast enough to run thread-count determinism 3x per test.
    Small,
    /// ~1000 files: the matrix's "medium synthetic" scenario.
    Medium,
    /// ~3000 files: the matrix's "large synthetic" scenario (bench/release only).
    Large,
}

impl SyntheticSize {
    /// Files generated per language; four languages are emitted, so the total
    /// file count is `4 * files_per_lang()`.
    pub fn files_per_lang(self) -> usize {
        match self {
            SyntheticSize::Small => 15,
            SyntheticSize::Medium => 250,
            SyntheticSize::Large => 750,
        }
    }
}

/// Files-per-language for the original 0.19 `scan_synthetic_480_files`
/// benchmark (120 * 4 languages = 480 files). Kept as its own constant so that
/// existing benchmark stays exactly reproducible while the v0.20 matrix adds
/// new sizes alongside it.
pub const LEGACY_480_FILES_PER_LANG: usize = 120;

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

/// Builds a synthetic multi-language repository (Rust/TypeScript/Python/Go)
/// with `files_per_lang` files per language, in a fresh temp directory. Use
/// [`SyntheticSize::files_per_lang`] for the v0.20 matrix sizes, or
/// [`LEGACY_480_FILES_PER_LANG`] to reproduce the original 0.19 fixture.
pub fn build_synthetic_repo(files_per_lang: usize) -> TempDir {
    let dir = tempfile::tempdir().expect("create temp repo");
    let root = dir.path();

    // A go.mod lets the Go resolver attempt module-relative resolution.
    fs::write(root.join("go.mod"), "module benchmod\n\ngo 1.22\n").expect("write go.mod");

    for index in 0..files_per_lang {
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

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Initializes a git repository with a deterministic local identity, matching
/// the setup used by `tests/scan_changed_cache_cli.rs` so `--changed` scans
/// have a stable base commit to diff against.
pub fn init_git_repo(root: &Path) {
    git(root, &["init"]);
    git(root, &["checkout", "-B", "main"]);
    git(root, &["config", "user.email", "repopilot@example.com"]);
    git(root, &["config", "user.name", "RepoPilot Test"]);
}

/// Stages and commits everything in `root`.
pub fn commit_all(root: &Path, message: &str) {
    git(root, &["add", "."]);
    git(root, &["commit", "-m", message]);
}
