//! Standalone resolver workload: rustc --edition=2024 -O scripts/performance/jvm-scaling.rs -o target/jvm-scaling
//! Measure the executable with /usr/bin/time -l (macOS) or -v (Linux).
#[path = "../../src/graph/resolver/jvm.rs"]
mod jvm;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Instant;
fn normalize_path(path: &Path) -> PathBuf {
    path.to_path_buf()
}

fn workload(size: usize) -> (HashSet<PathBuf>, Vec<(String, Option<PathBuf>)>) {
    let mut files = HashSet::new();
    let mut imports = Vec::new();
    for i in 0..size {
        let target = PathBuf::from(format!(
            "/repo/m{}/src/main/kotlin/com/example/p{i}/Type{i}.kt",
            i % 64
        ));
        files.insert(target.clone());
        imports.push((format!("com.example.p{i}.Type{i}"), Some(target.clone())));
        imports.push((
            format!("com.example.p{i}.Type{i}.Companion.VALUE"),
            Some(target),
        ));
        imports.push((format!("third.party.p{i}.Missing{i}"), None));
    }
    files.insert(PathBuf::from(
        "/repo/a/src/main/kotlin/com/example/Ambiguous.kt",
    ));
    files.insert(PathBuf::from(
        "/repo/b/src/main/kotlin/com/example/Ambiguous.kt",
    ));
    imports.push(("com.example.Ambiguous".into(), None));
    (files, imports)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let size = args
        .next()
        .map(|value| value.parse::<usize>())
        .transpose()?
        .unwrap_or(3000);
    let algorithm = args.next().unwrap_or_else(|| "indexed".into());
    if !["scan", "indexed"].contains(&algorithm.as_str()) {
        return Err(format!("unknown algorithm {algorithm:?}; expected scan or indexed").into());
    }
    let start = Instant::now();
    let (files, imports) = workload(size);
    println!(
        "workload=sequential-v1 seed=none files={} imports={} construction_us={}",
        files.len(),
        imports.len(),
        start.elapsed().as_micros()
    );
    let start = Instant::now();
    let index = (algorithm == "indexed").then(|| jvm::JvmIndex::new(&files));
    println!(
        "algorithm={algorithm} index_build_us={}",
        start.elapsed().as_micros()
    );
    for pass in ["cold", "warm"] {
        let start = Instant::now();
        let actual = imports
            .iter()
            .map(|(raw, _)| match &index {
                Some(index) => index.resolve(raw, &["kt", "java"]),
                None => jvm::resolve_jvm(raw, &files, &["kt", "java"]),
            })
            .collect::<Vec<_>>();
        let micros = start.elapsed().as_micros();
        assert!(
            actual
                .iter()
                .zip(&imports)
                .all(|(a, (_, expected))| a == expected)
        );
        println!(
            "pass={pass} resolution_us={micros} resolved={} unresolved={}",
            actual.iter().filter(|p| p.is_some()).count(),
            actual.iter().filter(|p| p.is_none()).count()
        );
    }
    Ok(())
}
