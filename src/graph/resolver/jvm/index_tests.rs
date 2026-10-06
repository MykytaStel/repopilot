use super::{JvmIndex, resolve_jvm};
use std::collections::HashSet;
use std::path::PathBuf;

#[test]
fn indexed_candidates_match_inventory_scan_for_complete_mixed_workload() {
    let mut paths = Vec::new();
    let mut imports = Vec::new();
    for i in 0..128 {
        for (set, ext) in [("main", "kt"), ("test", "kt"), ("main", "java")] {
            paths.push(PathBuf::from(format!(
                "/repo/m{i}/src/{set}/com/example/p{i}/Type{i}.{ext}"
            )));
        }
        paths.push(PathBuf::from(format!(
            "/repo/src/main/mycom/example/p{i}/Boundary{i}.kt"
        )));
        imports.extend([
            format!("com.example.p{i}.Type{i}"),
            format!("com.example.p{i}.Type{i}.Companion.VALUE"),
            format!("third.party.p{i}.Missing{i}"),
            format!("com.example.p{i}.Boundary{i}"),
        ]);
    }
    paths.extend([
        PathBuf::from("/repo/demo/com/example/Ambiguous.kt"),
        PathBuf::from("/repo/prod/com/example/Ambiguous.kt"),
    ]);
    imports.extend([
        "com.example.Ambiguous".into(),
        "com.example.*".into(),
        "Bare".into(),
    ]);
    let forward = paths.iter().cloned().collect::<HashSet<_>>();
    let reverse = paths.iter().rev().cloned().collect::<HashSet<_>>();
    for extensions in [&["java"][..], &["kt"][..], &["kt", "java"][..]] {
        let expected = imports
            .iter()
            .map(|raw| resolve_jvm(raw, &forward, extensions))
            .collect::<Vec<_>>();
        for files in [&forward, &reverse] {
            let index = JvmIndex::new(files);
            for _ in 0..2 {
                let actual = imports
                    .iter()
                    .map(|raw| index.resolve(raw, extensions))
                    .collect::<Vec<_>>();
                assert_eq!(actual, expected);
            }
        }
    }
}
