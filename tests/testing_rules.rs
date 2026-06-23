use repopilot::audits::testing::source_without_test::SourceWithoutTestAudit;
use repopilot::audits::traits::ProjectAudit;
use repopilot::scan::config::ScanConfig;
use repopilot::scan::facts::{FileFacts, ScanFacts};
use std::path::PathBuf;

#[test]
fn reports_missing_test_folder() {
    let facts = ScanFacts {
        root_path: PathBuf::from("demo"),
        files: vec![file("src/payment.rs")],
        files_analyzed: 1,
        ..ScanFacts::default()
    };

    let findings = repopilot::audits::testing::missing_test_folder::MissingTestFolderAudit
        .audit(&facts, &ScanConfig::default());

    assert_eq!(findings[0].rule_id, "testing.missing-test-folder");
}

#[test]
fn source_without_test_recognizes_integration_test_counterpart() {
    let facts = ScanFacts {
        root_path: PathBuf::from("demo"),
        files: vec![file("src/report/writer.rs"), file("tests/report_writer.rs")],
        files_analyzed: 2,
        ..ScanFacts::default()
    };

    let findings = SourceWithoutTestAudit.audit(&facts, &ScanConfig::default());

    assert!(findings.is_empty());
}

#[test]
fn source_without_test_recognizes_subsystem_integration_tests() {
    let facts = ScanFacts {
        root_path: PathBuf::from("demo"),
        files: vec![
            file("src/explain/render.rs"),
            file("src/output/markdown.rs"),
            file("tests/explain_cli.rs"),
            file("tests/output_markdown.rs"),
        ],
        files_analyzed: 4,
        ..ScanFacts::default()
    };

    let findings = SourceWithoutTestAudit.audit(&facts, &ScanConfig::default());

    assert!(findings.is_empty());
}

#[test]
fn source_without_test_reports_uncovered_source_and_ignores_wrappers() {
    let facts = ScanFacts {
        root_path: PathBuf::from("demo"),
        files: vec![
            file("src/payment.rs"),
            file("src/lib.rs"),
            file("src/mod.rs"),
        ],
        files_analyzed: 3,
        ..ScanFacts::default()
    };

    let findings = SourceWithoutTestAudit.audit(&facts, &ScanConfig::default());

    assert_eq!(findings.len(), 1);
    assert_eq!(
        findings[0].evidence[0].path,
        PathBuf::from("src/payment.rs")
    );
}

// ── Layer 3: TypeScript declaration files ────────────────────────────────────

#[test]
fn declaration_d_ts_never_flagged() {
    for path in [
        "src/types/theme.d.ts",
        "src/types/react-native-vector-icons.d.ts",
        "src/global.d.ts",
        "src/module.d.mts",
        "src/cjs.d.cts",
    ] {
        let facts = scan_facts_with(ts_file(path));
        let findings = SourceWithoutTestAudit.audit(&facts, &ScanConfig::default());
        assert!(
            findings.is_empty(),
            "declaration file must not be flagged: {path}"
        );
    }
}

// ── Layer 2: excluded directories ────────────────────────────────────────────

#[test]
fn types_directory_not_flagged() {
    for path in [
        "src/types/colors.ts",
        "src/types/api.ts",
        "src/@types/env.ts",
    ] {
        let facts = scan_facts_with(ts_file(path));
        let findings = SourceWithoutTestAudit.audit(&facts, &ScanConfig::default());
        assert!(
            findings.is_empty(),
            "file in types/ must not be flagged: {path}"
        );
    }
}

#[test]
fn generated_directory_not_flagged() {
    for path in [
        "src/generated/graphql.ts",
        "src/__generated__/schema.ts",
        "src/gen/proto.ts",
        "src/codegen/types.ts",
    ] {
        let facts = scan_facts_with(ts_file(path));
        let findings = SourceWithoutTestAudit.audit(&facts, &ScanConfig::default());
        assert!(
            findings.is_empty(),
            "generated file must not be flagged: {path}"
        );
    }
}

#[test]
fn mocks_directory_not_flagged() {
    for path in ["src/__mocks__/api.ts", "src/mocks/userService.ts"] {
        let facts = scan_facts_with(ts_file(path));
        let findings = SourceWithoutTestAudit.audit(&facts, &ScanConfig::default());
        assert!(findings.is_empty(), "mock file must not be flagged: {path}");
    }
}

// ── Layer 4: low-signal wrapper filenames ─────────────────────────────────────

#[test]
fn barrel_index_files_not_flagged() {
    for path in [
        "src/theme/index.ts",
        "src/components/index.tsx",
        "src/utils/index.js",
        "src/api/index.jsx",
    ] {
        let facts = scan_facts_with(ts_file(path));
        let findings = SourceWithoutTestAudit.audit(&facts, &ScanConfig::default());
        assert!(
            findings.is_empty(),
            "barrel index file must not be flagged: {path}"
        );
    }
}

#[test]
fn tokens_and_constants_not_flagged() {
    for path in [
        "src/theme/tokens.ts",
        "src/design/tokens.js",
        "src/constants.ts",
        "src/shared/constants.js",
        "src/theme/colors.ts",
        "src/theme/theme.ts",
        "src/shared/enums.ts",
        "src/api.constants.ts",
        "src/theme.tokens.ts",
    ] {
        let facts = scan_facts_with(ts_file(path));
        let findings = SourceWithoutTestAudit.audit(&facts, &ScanConfig::default());
        assert!(
            findings.is_empty(),
            "constant/token file must not be flagged: {path}"
        );
    }
}

#[test]
fn type_only_filenames_not_flagged() {
    for path in [
        "src/types.ts",
        "src/user.types.ts",
        "src/api/response.type.ts",
    ] {
        let facts = scan_facts_with(ts_file(path));
        let findings = SourceWithoutTestAudit.audit(&facts, &ScanConfig::default());
        assert!(
            findings.is_empty(),
            "type-only file must not be flagged: {path}"
        );
    }
}

#[test]
fn config_filenames_not_flagged() {
    for path in [
        "src/api.config.ts",
        "src/database.config.js",
        "src/vite.config.ts",
        "src/vitest.config.ts",
        "src/app.config.tsx",
        "src/build.config.mjs",
    ] {
        let facts = scan_facts_with(ts_file(path));
        let findings = SourceWithoutTestAudit.audit(&facts, &ScanConfig::default());
        assert!(
            findings.is_empty(),
            "config file must not be flagged: {path}"
        );
    }
}

#[test]
fn ts_js_entrypoints_not_flagged() {
    for path in ["src/main.ts", "src/main.tsx", "src/main.js", "src/main.jsx"] {
        let facts = scan_facts_with(ts_file(path));
        let findings = SourceWithoutTestAudit.audit(&facts, &ScanConfig::default());
        assert!(
            findings.is_empty(),
            "entrypoint file must not be flagged: {path}"
        );
    }
}

#[test]
fn python_init_and_infra_not_flagged() {
    for path in [
        "src/module/__init__.py",
        "src/conftest.py",
        "setup.py",
        "src/settings.py",
    ] {
        let facts = ScanFacts {
            root_path: PathBuf::from("."),
            files: vec![FileFacts {
                path: PathBuf::from(path),
                language: Some("Python".to_string()),
                non_empty_lines: 1,
                branch_count: 0,
                imports: Vec::new(),
                content: None,
                has_inline_tests: false,
                in_executable_package: false,
                deferred_imports: Vec::new(),
            }],
            files_analyzed: 1,
            ..ScanFacts::default()
        };
        let findings = SourceWithoutTestAudit.audit(&facts, &ScanConfig::default());
        assert!(
            findings.is_empty(),
            "Python infra file must not be flagged: {path}"
        );
    }
}

#[test]
fn regular_service_file_still_flagged() {
    let facts = scan_facts_with(ts_file("src/api/userService.ts"));
    let findings = SourceWithoutTestAudit.audit(&facts, &ScanConfig::default());
    assert_eq!(
        findings.len(),
        1,
        "regular service file with no test must still be flagged"
    );
}

fn scan_facts_with(file: FileFacts) -> ScanFacts {
    ScanFacts {
        root_path: PathBuf::from("."),
        files: vec![file],
        files_analyzed: 1,
        ..ScanFacts::default()
    }
}

fn ts_file(path: &str) -> FileFacts {
    FileFacts {
        path: PathBuf::from(path),
        language: Some("TypeScript".to_string()),
        non_empty_lines: 1,
        branch_count: 0,
        imports: Vec::new(),
        content: None,
        has_inline_tests: false,
        in_executable_package: false,
        deferred_imports: Vec::new(),
    }
}

fn file(path: &str) -> FileFacts {
    FileFacts {
        path: PathBuf::from(path),
        language: Some("Rust".to_string()),
        non_empty_lines: 1,
        branch_count: 0,
        imports: Vec::new(),
        content: Some("pub fn value() {}\n".to_string()),
        has_inline_tests: false,
        in_executable_package: false,
        deferred_imports: Vec::new(),
    }
}
