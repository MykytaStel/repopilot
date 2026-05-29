use std::path::{Component, Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ArchitecturePathScope {
    Production,
    Test,
    Fixture,
    Documentation,
    Example,
    Generated,
    Vendor,
    BuildOutput,
    Tooling,
}

const TOOLING_ARCHITECTURE_COMPONENTS: &[&str] = &[".git", ".repopilot"];

const BUILD_OUTPUT_ARCHITECTURE_COMPONENTS: &[&str] = &[
    ".next",
    ".nuxt",
    ".turbo",
    "build",
    "coverage",
    "deriveddata",
    "dist",
    "out",
    "target",
];

const VENDOR_ARCHITECTURE_COMPONENTS: &[&str] = &["node_modules", "pods", "vendor"];

const GENERATED_ARCHITECTURE_COMPONENTS: &[&str] = &["generated"];

const FIXTURE_ARCHITECTURE_COMPONENTS: &[&str] = &[
    "__fixtures__",
    "__mocks__",
    "__snapshots__",
    "fixture",
    "fixtures",
    "mock",
    "mocks",
    "snapshot",
    "snapshots",
];

const TEST_ARCHITECTURE_COMPONENTS: &[&str] = &["__tests__", "spec", "specs", "test", "tests"];

const DOCUMENTATION_ARCHITECTURE_COMPONENTS: &[&str] = &["doc", "docs"];

const EXAMPLE_ARCHITECTURE_COMPONENTS: &[&str] = &["example", "examples"];

pub(super) fn classify_architecture_path(path: &Path) -> ArchitecturePathScope {
    if has_blocked_path_component(path, TOOLING_ARCHITECTURE_COMPONENTS) {
        return ArchitecturePathScope::Tooling;
    }
    if has_blocked_path_component(path, BUILD_OUTPUT_ARCHITECTURE_COMPONENTS) {
        return ArchitecturePathScope::BuildOutput;
    }
    if has_blocked_path_component(path, VENDOR_ARCHITECTURE_COMPONENTS) {
        return ArchitecturePathScope::Vendor;
    }
    if has_blocked_path_component(path, DOCUMENTATION_ARCHITECTURE_COMPONENTS) {
        return ArchitecturePathScope::Documentation;
    }
    if has_blocked_path_component(path, EXAMPLE_ARCHITECTURE_COMPONENTS) {
        return ArchitecturePathScope::Example;
    }
    if has_blocked_path_component(path, FIXTURE_ARCHITECTURE_COMPONENTS) {
        return ArchitecturePathScope::Fixture;
    }
    if has_blocked_path_component(path, TEST_ARCHITECTURE_COMPONENTS) || is_test_file_name(path) {
        return ArchitecturePathScope::Test;
    }
    if has_blocked_path_component(path, GENERATED_ARCHITECTURE_COMPONENTS)
        || is_generated_file_name(path)
    {
        return ArchitecturePathScope::Generated;
    }

    ArchitecturePathScope::Production
}

fn has_blocked_path_component(path: &Path, blocked_components: &[&str]) -> bool {
    path.components().any(|component| {
        let Component::Normal(value) = component else {
            return false;
        };

        let Some(value) = value.to_str() else {
            return false;
        };

        blocked_components
            .iter()
            .any(|blocked| value.eq_ignore_ascii_case(blocked))
    })
}

fn is_test_file_name(path: &Path) -> bool {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();

    has_any_case_insensitive_suffix(
        file_name,
        &[
            ".test.ts",
            ".test.tsx",
            ".test.js",
            ".test.jsx",
            ".spec.ts",
            ".spec.tsx",
            ".spec.js",
            ".spec.jsx",
            "_test.rs",
            "_test.go",
        ],
    )
}

fn is_generated_file_name(path: &Path) -> bool {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();

    has_any_case_insensitive_suffix(
        file_name,
        &[
            ".generated.ts",
            ".generated.tsx",
            ".generated.js",
            ".generated.jsx",
            ".generated.rs",
            ".gen.ts",
            ".gen.tsx",
            ".gen.js",
            ".gen.jsx",
            ".gen.rs",
        ],
    )
}

fn has_any_case_insensitive_suffix(value: &str, suffixes: &[&str]) -> bool {
    suffixes.iter().any(|suffix| {
        value.len() >= suffix.len()
            && value[value.len() - suffix.len()..].eq_ignore_ascii_case(suffix)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn accepts_production_source_paths() {
        assert_eq!(
            classify_architecture_path(Path::new(
                "./src/features/payments/checkout/session/domain/handler.ts"
            )),
            ArchitecturePathScope::Production
        );
    }

    #[test]
    fn rejects_rule_fixture_paths() {
        assert_ne!(
            classify_architecture_path(Path::new(
                "./tests/fixtures/rules/security.secret-candidate/true_positive_env_value/src/config.ts"
            )),
            ArchitecturePathScope::Production
        );
    }

    #[test]
    fn rejects_test_corpora_and_test_files() {
        for path in [
            "./tests/unit/features/payments/session/service.ts",
            "./src/features/payments/session/service.test.ts",
            "./src/features/payments/session/service.spec.ts",
        ] {
            assert_ne!(
                classify_architecture_path(Path::new(path)),
                ArchitecturePathScope::Production,
                "{path} should not be treated as product architecture"
            );
        }
    }

    #[test]
    fn rejects_docs_examples_generated_vendor_and_build_output() {
        for path in [
            "./docs/reference/api/v1/generated/client/config.ts",
            "./examples/react-native/deep/sample/src/App.tsx",
            "./src/generated/openapi/client/v1/types.generated.ts",
            "./vendor/company/package/deep/source/file.ts",
            "./target/debug/build/package/out/generated.rs",
            "./dist/assets/js/chunks/deep/file.js",
        ] {
            assert!(
                !matches!(
                    classify_architecture_path(Path::new(path)),
                    ArchitecturePathScope::Production
                ),
                "{path} should not be treated as product architecture"
            );
        }
    }
}
