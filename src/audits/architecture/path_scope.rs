use std::path::{Component, Path};

const NON_PRODUCTION_ARCHITECTURE_COMPONENTS: &[&str] = &[
    ".git",
    ".next",
    ".nuxt",
    ".repopilot",
    ".turbo",
    "__fixtures__",
    "__mocks__",
    "__snapshots__",
    "__tests__",
    "build",
    "coverage",
    "deriveddata",
    "dist",
    "doc",
    "docs",
    "example",
    "examples",
    "fixture",
    "fixtures",
    "generated",
    "mock",
    "mocks",
    "node_modules",
    "out",
    "pods",
    "snapshot",
    "snapshots",
    "spec",
    "specs",
    "target",
    "test",
    "tests",
    "vendor",
];

/// Returns true when a path should be considered product/source architecture.
///
/// Architecture heuristics should not treat rule fixtures, test corpora, docs,
/// generated code, vendor trees, or build output as production structure. Those
/// paths may be intentionally deep or unusual because they describe scenarios,
/// not product module boundaries.
pub fn is_production_architecture_candidate(path: &Path) -> bool {
    !has_blocked_path_component(path, NON_PRODUCTION_ARCHITECTURE_COMPONENTS)
        && !is_test_or_generated_file_name(path)
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

fn is_test_or_generated_file_name(path: &Path) -> bool {
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
        assert!(is_production_architecture_candidate(Path::new(
            "./src/features/payments/checkout/session/domain/handler.ts"
        )));
    }

    #[test]
    fn rejects_rule_fixture_paths() {
        assert!(!is_production_architecture_candidate(Path::new(
            "./tests/fixtures/rules/security.secret-candidate/true_positive_env_value/src/config.ts"
        )));
    }

    #[test]
    fn rejects_test_corpora_and_test_files() {
        assert!(!is_production_architecture_candidate(Path::new(
            "./tests/unit/features/payments/session/service.ts"
        )));
        assert!(!is_production_architecture_candidate(Path::new(
            "./src/features/payments/session/service.test.ts"
        )));
        assert!(!is_production_architecture_candidate(Path::new(
            "./src/features/payments/session/service.spec.ts"
        )));
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
                !is_production_architecture_candidate(Path::new(path)),
                "{path} should not be treated as product architecture"
            );
        }
    }
}
