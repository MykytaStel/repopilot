use super::*;
use crate::scan::config::ScanConfig;
use crate::scan::facts::{FileFacts, ScanFacts};
use std::io::Write;
use tempfile::tempdir;

fn make_file_facts(path: std::path::PathBuf) -> FileFacts {
    FileFacts {
        path,
        language: Some("TypeScript".to_string()),
        non_empty_lines: 2,
        branch_count: 0,
        imports: vec![],
        content: None,
        has_inline_tests: false,
    }
}

// ── VarDeclarationAudit ───────────────────────────────────────────────────

#[test]
fn var_declaration_flagged() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("utils.ts");
    write!(
        std::fs::File::create(&file_path).unwrap(),
        "var x = 1;\nconst y = 2;\n"
    )
    .unwrap();
    let mut facts = ScanFacts {
        root_path: dir.path().to_path_buf(),
        ..ScanFacts::default()
    };
    facts.files.push(make_file_facts(file_path));
    let findings = VarDeclarationAudit.audit(&facts, &ScanConfig::default());
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "framework.js.var-declaration");
}

#[test]
fn var_inside_identifier_not_flagged() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("utils.ts");
    // "typeVar", "localStorage", "varName" should NOT trigger
    write!(
        std::fs::File::create(&file_path).unwrap(),
        "const typeVar = 1;\nconst localStorage = {{}};\nconst varName = 3;\n"
    )
    .unwrap();
    let mut facts = ScanFacts {
        root_path: dir.path().to_path_buf(),
        ..ScanFacts::default()
    };
    facts.files.push(make_file_facts(file_path));
    let findings = VarDeclarationAudit.audit(&facts, &ScanConfig::default());
    assert!(
        findings.is_empty(),
        "identifiers containing 'var' must not be flagged"
    );
}

#[test]
fn var_in_test_file_skipped() {
    let dir = tempdir().unwrap();
    let test_dir = dir.path().join("__tests__");
    std::fs::create_dir(&test_dir).unwrap();
    let file_path = test_dir.join("utils.test.ts");
    writeln!(std::fs::File::create(&file_path).unwrap(), "var x = 1;").unwrap();
    let mut facts = ScanFacts {
        root_path: dir.path().to_path_buf(),
        ..ScanFacts::default()
    };
    facts.files.push(make_file_facts(file_path));
    let findings = VarDeclarationAudit.audit(&facts, &ScanConfig::default());
    assert!(findings.is_empty());
}

#[test]
fn no_var_no_finding() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("utils.ts");
    write!(
        std::fs::File::create(&file_path).unwrap(),
        "const x = 1;\nlet y = 2;\n"
    )
    .unwrap();
    let mut facts = ScanFacts {
        root_path: dir.path().to_path_buf(),
        ..ScanFacts::default()
    };
    facts.files.push(make_file_facts(file_path));
    let findings = VarDeclarationAudit.audit(&facts, &ScanConfig::default());
    assert!(findings.is_empty());
}

// ── ConsoleLogAudit ───────────────────────────────────────────────────────

#[test]
fn console_log_flagged() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("Screen.tsx");
    write!(
        std::fs::File::create(&file_path).unwrap(),
        "const x = 1;\nconsole.log(x);\n"
    )
    .unwrap();
    let mut facts = ScanFacts {
        root_path: dir.path().to_path_buf(),
        ..ScanFacts::default()
    };
    facts.files.push(FileFacts {
        path: file_path,
        language: Some("TypeScript React".to_string()),
        non_empty_lines: 2,
        branch_count: 0,
        imports: vec![],
        content: None,
        has_inline_tests: false,
    });
    let findings = ConsoleLogAudit.audit(&facts, &ScanConfig::default());
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "framework.js.console-log");
}

#[test]
fn console_log_in_comment_not_flagged() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("utils.ts");
    write!(
        std::fs::File::create(&file_path).unwrap(),
        "// console.log(debug)\nconst x = 1;\n"
    )
    .unwrap();
    let mut facts = ScanFacts {
        root_path: dir.path().to_path_buf(),
        ..ScanFacts::default()
    };
    facts.files.push(make_file_facts(file_path));
    let findings = ConsoleLogAudit.audit(&facts, &ScanConfig::default());
    assert!(findings.is_empty());
}
