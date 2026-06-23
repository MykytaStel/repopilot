use super::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn detects_large_index_barrel_file() {
    let temp = tempdir().expect("temp dir");
    let file_path = temp.path().join("index.ts");

    fs::write(
        &file_path,
        [
            "export { A } from './a';",
            "export { B } from './b';",
            "export { C } from './c';",
            "export { D } from './d';",
            "export { E } from './e';",
            "export { F } from './f';",
            "export { G } from './g';",
            "export { H } from './h';",
        ]
        .join("\n"),
    )
    .expect("write file");

    let findings = BarrelFileRiskAudit.audit(&facts_for_file(file_path), &ScanConfig::default());

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, RULE_ID);
    assert_eq!(findings[0].severity, Severity::Low);
    assert_eq!(findings[0].evidence[0].line_start, 1);
}

#[test]
fn detects_wildcard_heavy_barrel_file_as_medium() {
    let temp = tempdir().expect("temp dir");
    let file_path = temp.path().join("index.ts");

    fs::write(
        &file_path,
        [
            "export * from './a';",
            "export * from './b';",
            "export * from './c';",
            "export * from './d';",
            "export * from './e';",
            "export * from './f';",
        ]
        .join("\n"),
    )
    .expect("write file");

    let findings = BarrelFileRiskAudit.audit(&facts_for_file(file_path), &ScanConfig::default());

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Medium);
}

#[test]
fn ignores_small_index_barrel_file() {
    let temp = tempdir().expect("temp dir");
    let file_path = temp.path().join("index.ts");

    fs::write(
        &file_path,
        [
            "export { A } from './a';",
            "export { B } from './b';",
            "export { C } from './c';",
        ]
        .join("\n"),
    )
    .expect("write file");

    let findings = BarrelFileRiskAudit.audit(&facts_for_file(file_path), &ScanConfig::default());

    assert!(findings.is_empty());
}

#[test]
fn detects_broad_barrel_filenames() {
    let temp = tempdir().expect("temp dir");
    let file_path = temp.path().join("exports.ts");

    fs::write(
        &file_path,
        [
            "export * from './a';",
            "export * from './b';",
            "export * from './c';",
            "export * from './d';",
        ]
        .join("\n"),
    )
    .expect("write file");

    let findings = BarrelFileRiskAudit.audit(&facts_for_file(file_path), &ScanConfig::default());

    assert_eq!(findings.len(), 1);
}

#[test]
fn ignores_non_index_file() {
    let temp = tempdir().expect("temp dir");
    let file_path = temp.path().join("utils.ts");

    fs::write(
        &file_path,
        [
            "export * from './a';",
            "export * from './b';",
            "export * from './c';",
            "export * from './d';",
        ]
        .join("\n"),
    )
    .expect("write file");

    let findings = BarrelFileRiskAudit.audit(&facts_for_file(file_path), &ScanConfig::default());

    assert!(findings.is_empty());
}

#[test]
fn ignores_commented_exports() {
    let temp = tempdir().expect("temp dir");
    let file_path = temp.path().join("index.ts");

    fs::write(
        &file_path,
        [
            "// export * from './a';",
            "// export * from './b';",
            "// export * from './c';",
            "export { Real } from './real';",
        ]
        .join("\n"),
    )
    .expect("write file");

    let findings = BarrelFileRiskAudit.audit(&facts_for_file(file_path), &ScanConfig::default());

    assert!(findings.is_empty());
}

fn facts_for_file(path: std::path::PathBuf) -> ScanFacts {
    ScanFacts {
        files: vec![FileFacts {
            path,
            language: None,
            non_empty_lines: 1,
            branch_count: 0,
            imports: vec![],
            content: None,
            has_inline_tests: false,
            in_executable_package: false,
            deferred_imports: Vec::new(),
        }],
        ..ScanFacts::default()
    }
}
