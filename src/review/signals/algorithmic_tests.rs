use super::algorithmic::{AlgorithmicKind, AlgorithmicSignal, detect_algorithmic};
use crate::review::diff::{ChangeStatus, ChangedFile, ChangedRange};
use crate::review::signals::content::ReviewSource;
use std::path::PathBuf;

fn source(content: &str) -> ReviewSource {
    ReviewSource::new(content.to_string(), Some("Rust".to_string()))
}

/// A modified file whose changed range covers the whole source.
fn changed(path: &str) -> ChangedFile {
    ChangedFile {
        path: PathBuf::from(path),
        status: ChangeStatus::Modified,
        ranges: vec![ChangedRange {
            start: 1,
            end: 100_000,
        }],
        hunks: Vec::new(),
    }
}

fn kinds(signals: &[AlgorithmicSignal]) -> Vec<AlgorithmicKind> {
    signals.iter().map(|signal| signal.kind).collect()
}

#[test]
fn flags_nested_loop_introduced() {
    let pre =
        source("fn process(items: &[i32]) {\n    for i in items {\n        let _ = i;\n    }\n}\n");
    let post = source(
        "fn process(items: &[i32]) {\n    for i in items {\n        for j in items {\n            let _ = i + j;\n        }\n    }\n}\n",
    );
    let file = changed("src/process.rs");
    let signals = detect_algorithmic(&file, Some(&pre), Some(&post));
    assert!(
        kinds(&signals).contains(&AlgorithmicKind::NestedLoopIntroduced),
        "{signals:?}"
    );
}

#[test]
fn flags_complexity_increased() {
    let pre = source("fn check(a: bool) {\n    if a {\n        run();\n    }\n}\n");
    let post = source(
        "fn check(a: bool, b: bool, c: bool) {\n    if a {\n        if b {\n            if c {\n                run();\n            }\n        }\n    }\n}\n",
    );
    let file = changed("src/check.rs");
    let signals = detect_algorithmic(&file, Some(&pre), Some(&post));
    let complexity: Vec<_> = signals
        .iter()
        .filter(|signal| signal.kind == AlgorithmicKind::ComplexityIncreased)
        .collect();
    assert_eq!(complexity.len(), 1, "{signals:?}");
    assert!(
        complexity[0].detail.contains("1 → 3"),
        "{}",
        complexity[0].detail
    );
}

#[test]
fn flags_recursion_introduced() {
    let pre = source("fn fib(n: u64) -> u64 {\n    n\n}\n");
    let post = source("fn fib(n: u64) -> u64 {\n    fib(n - 1) + fib(n - 2)\n}\n");
    let file = changed("src/fib.rs");
    let signals = detect_algorithmic(&file, Some(&pre), Some(&post));
    assert!(
        kinds(&signals).contains(&AlgorithmicKind::RecursionIntroduced),
        "{signals:?}"
    );
}

#[test]
fn flags_function_grew() {
    let pre = source("fn build() {\n    let x = 1;\n}\n");
    let mut body = String::from("fn build() {\n");
    for index in 0..60 {
        body.push_str(&format!("    let v{index} = {index};\n"));
    }
    body.push_str("}\n");
    let post = source(&body);
    let file = changed("src/build.rs");
    let signals = detect_algorithmic(&file, Some(&pre), Some(&post));
    assert!(
        kinds(&signals).contains(&AlgorithmicKind::FunctionGrew),
        "{signals:?}"
    );
}

#[test]
fn flags_new_function_with_nested_loop() {
    let post = source(
        "fn matrix(n: usize) {\n    for i in 0..n {\n        for j in 0..n {\n            let _ = i * j;\n        }\n    }\n}\n",
    );
    let file = changed("src/matrix.rs");
    // No pre source at all (whole file is new), so the loop is "introduced".
    let signals = detect_algorithmic(&file, None, Some(&post));
    assert!(
        kinds(&signals).contains(&AlgorithmicKind::NestedLoopIntroduced),
        "{signals:?}"
    );
}

#[test]
fn silent_when_shape_unchanged() {
    let code =
        "fn calc(a: i32) -> i32 {\n    if a > 0 {\n        a\n    } else {\n        0\n    }\n}\n";
    let pre = source(code);
    let post = source(code);
    let file = changed("src/calc.rs");
    let signals = detect_algorithmic(&file, Some(&pre), Some(&post));
    assert!(signals.is_empty(), "{signals:?}");
}

#[test]
fn skips_test_files() {
    let post = source(
        "fn helper() {\n    for i in 0..3 {\n        for j in 0..3 {\n            let _ = i + j;\n        }\n    }\n}\n",
    );
    let file = changed("src/calc_test.rs");
    let signals = detect_algorithmic(&file, None, Some(&post));
    assert!(signals.is_empty(), "{signals:?}");
}

#[test]
fn ignores_functions_outside_changed_range() {
    let post = source(
        "fn untouched(n: usize) {\n    for i in 0..n {\n        for j in 0..n {\n            let _ = i * j;\n        }\n    }\n}\n",
    );
    let mut file = changed("src/untouched.rs");
    // The changed range is far from the function's lines (1..=7).
    file.ranges = vec![ChangedRange {
        start: 100,
        end: 101,
    }];
    let signals = detect_algorithmic(&file, None, Some(&post));
    assert!(signals.is_empty(), "{signals:?}");
}
