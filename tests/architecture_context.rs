use repopilot::analysis::{ArchitectureClassifier, FileRole, LanguageFamily, ModuleKind};
use repopilot::scan::config::ScanConfig;
use repopilot::scan::facts::FileFacts;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[test]
fn test_default_module_mappings() {
    let config = ScanConfig::default();
    let classifier = ArchitectureClassifier::new(&config.module_mappings);

    let test_cases = vec![
        (
            "src/features/billing/domain/service.rs",
            ModuleKind::Domain,
            FileRole::Production,
        ),
        (
            "src/ui/components/Button.tsx",
            ModuleKind::Ui,
            FileRole::Production,
        ),
        (
            "src/infra/db/client.go",
            ModuleKind::Infrastructure,
            FileRole::Production,
        ),
        (
            "src/shared/utils/date.ts",
            ModuleKind::Shared,
            FileRole::Production,
        ),
        (
            "tests/integration/test_helper.py",
            ModuleKind::Unknown,
            FileRole::Test,
        ),
        (
            "src/cli/commands/scan.rs",
            ModuleKind::Cli,
            FileRole::Production,
        ),
        (
            "docs/index.md",
            ModuleKind::Unknown,
            FileRole::Documentation,
        ),
        (
            "tests/fixtures/sample.json",
            ModuleKind::Unknown,
            FileRole::Fixture,
        ),
        (
            "src/generated/types.ts",
            ModuleKind::Unknown,
            FileRole::Generated,
        ),
        ("cargo.toml", ModuleKind::Unknown, FileRole::Config),
    ];

    for (path_str, expected_module, expected_role) in test_cases {
        let file = FileFacts {
            path: PathBuf::from(path_str),
            language: None,
            non_empty_lines: 10,
            branch_count: 0,
            imports: vec![],
            content: None,
            has_inline_tests: false,
            in_executable_package: false,
            deferred_imports: Vec::new(),
        };

        let context = classifier.classify(&file);
        assert_eq!(
            context.module_kind, expected_module,
            "Path '{}' classified as {:?}, expected {:?}",
            path_str, context.module_kind, expected_module
        );
        assert_eq!(
            context.file_role, expected_role,
            "Path '{}' classified as {:?}, expected {:?}",
            path_str, context.file_role, expected_role
        );
    }
}

#[test]
fn test_custom_module_mappings() {
    let mut custom_mappings = BTreeMap::new();
    custom_mappings.insert("domain".to_string(), vec!["**/core/models/**".to_string()]);
    custom_mappings.insert("ui".to_string(), vec!["**/views/**".to_string()]);

    let classifier = ArchitectureClassifier::new(&custom_mappings);

    let domain_file = FileFacts {
        path: PathBuf::from("src/core/models/user.rs"),
        language: None,
        non_empty_lines: 10,
        branch_count: 0,
        imports: vec![],
        content: None,
        has_inline_tests: false,
        in_executable_package: false,
        deferred_imports: Vec::new(),
    };
    let context = classifier.classify(&domain_file);
    assert_eq!(context.module_kind, ModuleKind::Domain);

    let ui_file = FileFacts {
        path: PathBuf::from("src/views/MainScreen.tsx"),
        language: None,
        non_empty_lines: 10,
        branch_count: 0,
        imports: vec![],
        content: None,
        has_inline_tests: false,
        in_executable_package: false,
        deferred_imports: Vec::new(),
    };
    let context = classifier.classify(&ui_file);
    assert_eq!(context.module_kind, ModuleKind::Ui);

    let unknown_file = FileFacts {
        path: PathBuf::from("src/features/billing/domain/service.rs"),
        language: None,
        non_empty_lines: 10,
        branch_count: 0,
        imports: vec![],
        content: None,
        has_inline_tests: false,
        in_executable_package: false,
        deferred_imports: Vec::new(),
    };
    let context = classifier.classify(&unknown_file);
    assert_eq!(context.module_kind, ModuleKind::Unknown);
}

#[test]
fn test_language_families() {
    let config = ScanConfig::default();
    let classifier = ArchitectureClassifier::new(&config.module_mappings);

    let test_cases = vec![
        ("src/main.rs", "Rust", LanguageFamily::CurlyBrace),
        ("src/index.ts", "TypeScript", LanguageFamily::CurlyBrace),
        ("src/app.py", "Python", LanguageFamily::Python),
        ("src/main.go", "Go", LanguageFamily::Go),
        ("script.sh", "Shell", LanguageFamily::Shell),
        ("config.json", "JSON", LanguageFamily::Markup),
        ("readme.md", "Markdown", LanguageFamily::Markup),
    ];

    for (path_str, language_str, expected_family) in test_cases {
        let file = FileFacts {
            path: PathBuf::from(path_str),
            language: Some(language_str.to_string()),
            non_empty_lines: 10,
            branch_count: 0,
            imports: vec![],
            content: None,
            has_inline_tests: false,
            in_executable_package: false,
            deferred_imports: Vec::new(),
        };

        let context = classifier.classify(&file);
        assert_eq!(
            context.language_family, expected_family,
            "Language '{}' classified as {:?}, expected {:?}",
            language_str, context.language_family, expected_family
        );
    }
}
