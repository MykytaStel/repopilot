//! Per-language path and naming conventions.
//!
//! The cross-language rules (test directories like `tests/`/`__tests__/`,
//! the `tests_` filename prefix, build-tooling paths) stay in the context
//! classifier's helpers; what varies by language lives here: test-file
//! naming, whether the `test_` prefix convention applies, test-support
//! recognizers, and app-entrypoint content probes. The entrypoint *filename*
//! list stays shared — it has always matched union-style across languages.

use super::frontend_for_kind;
use super::generic::GENERIC;
use crate::audits::context::LanguageKind;
use std::path::Path;

/// A test-support recognizer: production modules (or source sets) whose
/// panics/asserts are test plumbing. The reason string is role evidence and
/// must stay stable per convention.
pub struct TestSupportConvention {
    pub(crate) matches: fn(&Path) -> bool,
    pub(crate) reason: &'static str,
}

pub struct PathConventions {
    /// Language-specific test-file rule over the lowercased file name.
    pub(crate) test_file_name: fn(&str) -> bool,
    /// Whether the cross-language `test_` filename prefix marks a test file.
    /// False for Rust, where it collides with production modules
    /// (`test_edges.rs`).
    pub(crate) test_prefix_marks_test: bool,
    /// Test-support recognizer, when the language has one.
    pub(crate) test_support: Option<&'static TestSupportConvention>,
    /// Content probe for app entrypoints (`fn main(`, `if __name__ …`).
    pub(crate) entrypoint_content: Option<fn(&str) -> bool>,
}

pub(crate) static GENERIC_CONVENTIONS: PathConventions = PathConventions {
    test_file_name: |_| false,
    test_prefix_marks_test: true,
    test_support: None,
    entrypoint_content: None,
};

/// Gradle/source-set shapes dedicating a whole module or source set to test
/// doubles (`core/testing/src/main/...`, `module/src/testFixtures/...`); a
/// package namespace component like `.../com/example/testing/...` is
/// ordinary production code. Shared by the Java, Kotlin, and C# frontends.
pub(crate) static MANAGED_TEST_SUPPORT: TestSupportConvention = TestSupportConvention {
    matches: |path| {
        let components: Vec<String> = path
            .to_string_lossy()
            .split(['/', '\\'])
            .map(|component| component.trim().to_lowercase())
            .collect();

        components.windows(3).any(|window| {
            matches!(window[0].as_str(), "testing" | "testsupport")
                && window[1] == "src"
                && window[2] == "main"
        }) || components
            .windows(2)
            .any(|window| window[0] == "src" && window[1] == "testfixtures")
    },
    reason: "recognized managed-language test-support source-set path",
};

/// The conventions for the language detection assigns to `path`, falling
/// back to the generic frontend's conventions.
pub(crate) fn conventions_for_path(path: &Path) -> &'static PathConventions {
    crate::knowledge::language::detect_language_for_path(path)
        .and_then(super::frontend_for_label)
        .map(|frontend| frontend.conventions)
        .unwrap_or(GENERIC.conventions)
}

/// The conventions for a context-classifier kind.
pub(crate) fn conventions_for_kind(kind: LanguageKind) -> &'static PathConventions {
    frontend_for_kind(kind).conventions
}

/// Every distinct conventions table, for guard tests.
#[cfg(test)]
pub(crate) fn all_conventions() -> Vec<(&'static str, &'static PathConventions)> {
    super::all_frontends()
        .iter()
        .map(|frontend| (frontend.id, frontend.conventions))
        .collect()
}
