use crate::audits::context::model::LanguageKind;
use std::path::Path;

pub fn push_unique<T: PartialEq>(values: &mut Vec<T>, value: T) {
    if !values.contains(&value) {
        values.push(value);
    }
}

pub fn normalize(value: &str) -> String {
    value.trim().to_lowercase()
}

pub fn path_contains_component(path: &Path, targets: &[&str]) -> bool {
    path.to_string_lossy().split(['/', '\\']).any(|component| {
        let normalized = normalize(component);
        targets.iter().any(|target| normalized == *target)
    })
}

pub fn is_pascal_case(value: &str) -> bool {
    value
        .chars()
        .next()
        .map(|character| character.is_uppercase())
        .unwrap_or(false)
}

pub fn is_js_or_ts(language: LanguageKind) -> bool {
    matches!(
        language,
        LanguageKind::TypeScript | LanguageKind::JavaScript
    )
}

/// Classifies a file as a *test file* purely by its path and name conventions.
///
/// Whether a file *contains* inline tests (Rust `#[cfg(test)] mod tests`, a
/// Python doctest, etc.) is a separate fact carried by
/// `FileFacts::has_inline_tests` and must NOT promote the file to a test role:
/// a production module that happens to carry an inline `#[cfg(test)]` block
/// still ships its production code and is imported as production. Conflating
/// the two made every Rust file with inline tests look like a test file, which
/// turned ordinary production-to-production imports into false
/// `architecture.test-leak` findings. The role is the file's purpose, decided
/// by location/name; the inline-test flag is an orthogonal coverage signal.
pub fn is_test_file(path: &Path) -> bool {
    let path_text = path.to_string_lossy().to_lowercase();
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_lowercase())
        .unwrap_or_default();

    path_text.starts_with("tests/")
        || path_text.starts_with("tests\\")
        || path_text.contains("/tests/")
        || path_text.contains("\\tests\\")
        || path_text.starts_with("fixtures/")
        || path_text.starts_with("fixtures\\")
        || path_text.contains("/fixtures/")
        || path_text.contains("\\fixtures\\")
        || path_text.contains("/__tests__/")
        || path_text.contains("\\__tests__\\")
        || file_name.ends_with(".test.ts")
        || file_name.ends_with(".test.tsx")
        || file_name.ends_with(".test.js")
        || file_name.ends_with(".test.jsx")
        || file_name.ends_with(".spec.ts")
        || file_name.ends_with(".spec.tsx")
        || file_name.ends_with(".spec.js")
        || file_name.ends_with(".spec.jsx")
        || file_name.ends_with("_test.go")
        || file_name.ends_with("_test.py")
        || file_name.ends_with("test.java")
        || file_name.ends_with("tests.java")
        || file_name.ends_with("test.kt")
        || file_name.ends_with("tests.kt")
        || file_name.ends_with("test.cs")
        || file_name.ends_with("tests.cs")
        // The `test_` prefix is a Python / JS test convention. It is NOT a Rust
        // one (Rust uses `tests/`, `#[cfg(test)]`, or plural `_tests.rs`), where
        // it instead collides with production modules like `test_edges.rs`.
        || (file_name.starts_with("test_") && !file_name.ends_with(".rs"))
        // Rust test modules carry no path component (a sibling `tests.rs` /
        // `tests_render.rs` / `*_tests.rs` pulled in via `#[cfg(test)] mod ...;`).
        // Only the *plural* forms are used for Rust test files; the singular
        // `*_test.rs` collides with production names (e.g. `source_without_test.rs`),
        // so it is deliberately omitted — real `*_test.rs` integration tests live
        // under `tests/` and are already matched by the path checks above.
        || file_name == "tests.rs"
        || file_name.ends_with("_tests.rs")
        || file_name.starts_with("tests_")
}

/// True for Rust *test-support* modules — `testutil.rs`, `test_utils.rs`,
/// `test_support.rs`, `test_helpers.rs` and singular variants. Unlike a test
/// file, this is a production module (compiled in normal builds, not behind
/// `#[cfg(test)]`), but its `panic!`/`unwrap` calls are assertion plumbing for
/// tests rather than production risk. It is exposed as the separate
/// `FileRole::TestSupport` so only opted-in rules (currently `rust.panic-risk`)
/// treat it specially; the file keeps its ordinary production role for every
/// other rule. An explicit allow list keeps the collision-prone `test_*` prefix
/// (the production `test_edges.rs` / `source_without_test.rs`) out.
pub fn is_test_support_file(path: &Path) -> bool {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_lowercase())
        .unwrap_or_default();

    matches!(
        file_name.as_str(),
        "testutil.rs"
            | "testutils.rs"
            | "test_util.rs"
            | "test_utils.rs"
            | "test_support.rs"
            | "test_helper.rs"
            | "test_helpers.rs"
    )
}

/// True for a managed-language *test-support module directory* — a source tree
/// dedicated to test doubles and helpers shared across modules. This is limited
/// to Gradle/source-set shapes that indicate a whole helper module/source set,
/// such as `core/testing/src/main/...`, `core/testsupport/src/main/...`, or
/// `module/src/testFixtures/...`; a package namespace component like
/// `app/src/main/kotlin/com/example/testing/...` is ordinary production code.
pub fn is_managed_test_support_path(path: &Path) -> bool {
    let components = normalized_components(path);

    components.windows(3).any(|window| {
        matches!(window[0].as_str(), "testing" | "testsupport")
            && window[1] == "src"
            && window[2] == "main"
    }) || components
        .windows(2)
        .any(|window| window[0] == "src" && window[1] == "testfixtures")
}

/// True for *build-tooling* sources — Gradle convention plugins and build logic
/// under `build-logic/` or `buildSrc/`. These configure the build and never ship
/// in the application, so a `throw`/`TODO()` there fails the build by design.
pub fn is_build_tooling_path(path: &Path) -> bool {
    path_contains_component(path, &["build-logic", "buildsrc"])
}

fn normalized_components(path: &Path) -> Vec<String> {
    path.to_string_lossy()
        .split(['/', '\\'])
        .map(normalize)
        .collect()
}

pub fn is_config_file(path: &Path) -> bool {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(normalize)
        .unwrap_or_default();

    matches!(
        file_name.as_str(),
        "package.json"
            | "tsconfig.json"
            | "vite.config.ts"
            | "vite.config.js"
            | "next.config.js"
            | "next.config.mjs"
            | "cargo.toml"
            | "cargo.lock"
            | "projectsettings.asset"
            | "dockerfile"
            | "containerfile"
            | "go.mod"
            | "go.sum"
            | "pyproject.toml"
            | "requirements.txt"
            | "build.gradle"
            | "settings.gradle"
            | "pom.xml"
    ) || (file_name.starts_with("appsettings") && file_name.ends_with(".json"))
}

pub fn is_generated_file(path: &Path, content: &str) -> bool {
    path_contains_component(
        path,
        &[
            "generated",
            "__generated__",
            "gen",
            "codegen",
            "target",
            "build",
        ],
    ) || content.contains("@generated")
        || content.contains("code generated")
        || content.contains("Code generated")
        || content.contains("Code Generated")
        || content.contains("generated by")
        || content.contains("Generated by")
        || content.contains("Generated By")
        || looks_like_vendored_bundle(content)
}

/// Vendored or generated JS/TS bundles — Emscripten/WASM glue, codegen output —
/// are not hand-maintained source, so their runtime constructs (e.g. an
/// Emscripten `process.exit` shim) are noise rather than the hazard a rule
/// targets. They announce themselves with an Emscripten runtime marker or a
/// bare, whole-file lint opt-out as the very first line (no rule list — a
/// signature hand-written code does not use).
fn looks_like_vendored_bundle(content: &str) -> bool {
    if content.contains("Emscripten Module") || content.contains("EMSCRIPTEN_") {
        return true;
    }
    content
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .is_some_and(|line| line == "/* eslint-disable */")
}

pub fn is_app_entrypoint(path: &Path, content: &str, language: LanguageKind) -> bool {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(normalize)
        .unwrap_or_default();

    matches!(
        file_name.as_str(),
        "main.rs"
            | "build.rs"
            | "main.go"
            | "main.py"
            | "app.py"
            | "program.cs"
            | "main.java"
            | "main.kt"
            | "index.ts"
            | "index.js"
            | "index.tsx"
            | "index.jsx"
            | "main.ts"
            | "main.js"
            | "main.tsx"
            | "main.jsx"
    ) || (language == LanguageKind::Python && content.contains("if __name__ == \"__main__\""))
        || (language == LanguageKind::Go && content.contains("func main("))
        || (language == LanguageKind::Rust && content.contains("fn main("))
}

#[cfg(test)]
mod tests {
    use super::{is_app_entrypoint, is_test_file, is_test_support_file, path_contains_component};
    use crate::audits::context::model::LanguageKind;
    use std::path::Path;

    #[test]
    fn test_support_allowlist_excludes_production_test_named_modules() {
        for support in [
            "crates/searcher/src/testutil.rs",
            "src/test_utils.rs",
            "src/test_support.rs",
            "src/test_helpers.rs",
        ] {
            assert!(is_test_support_file(Path::new(support)), "{support}");
            // A test-support module is NOT a test file — it keeps its production role.
            assert!(!is_test_file(Path::new(support)), "{support}");
        }
        // Production modules whose names merely resemble the `test_*` convention
        // must not be swept in.
        assert!(!is_test_support_file(Path::new("src/graph/test_edges.rs")));
        assert!(!is_test_support_file(Path::new(
            "src/audits/testing/source_without_test.rs"
        )));
    }

    #[test]
    fn entrypoints_recognized_by_filename_without_content() {
        // The import graph classifies nodes after per-file content has been
        // dropped, so entrypoint detection must work from the filename alone —
        // otherwise a Cargo build script (`fn main()` but content unavailable)
        // is wrongly reported as a dead module, and every Vite/React
        // `src/main.tsx` is treated as ordinary importable code.
        assert!(is_app_entrypoint(
            Path::new("build.rs"),
            "",
            LanguageKind::Rust
        ));
        assert!(is_app_entrypoint(
            Path::new("src/main.tsx"),
            "",
            LanguageKind::TypeScript
        ));
        assert!(is_app_entrypoint(
            Path::new("src/index.tsx"),
            "",
            LanguageKind::TypeScript
        ));
        // A regular library module is still not an entrypoint.
        assert!(!is_app_entrypoint(
            Path::new("src/state.rs"),
            "",
            LanguageKind::Rust
        ));
    }

    #[test]
    fn path_component_matching_handles_windows_separators() {
        assert!(path_contains_component(
            Path::new(r"tools\scripts\check.js"),
            &["scripts"],
        ));
        assert!(path_contains_component(
            Path::new(r"src\domain\model.rs"),
            &["domain"],
        ));
    }

    #[test]
    fn test_classification_covers_rust_test_modules_and_fixtures() {
        assert!(is_test_file(Path::new("src/behavioral_tests.rs")));
        assert!(is_test_file(Path::new("tests/fixtures/runtime/client.rs")));
        assert!(is_test_file(Path::new(r"fixtures\runtime\client.rs")));
        // Sibling Rust test modules pulled in via `#[cfg(test)] mod ...;`.
        assert!(is_test_file(Path::new("src/audits/foo/tests.rs")));
        assert!(is_test_file(Path::new(
            "src/audits/code_quality/rust_panic_risk/tests_render.rs"
        )));
    }

    #[test]
    fn inline_tests_do_not_make_a_production_file_a_test_file() {
        // A production module with an inline `#[cfg(test)] mod tests` block is
        // still production: its role must not depend on carrying inline tests.
        assert!(!is_test_file(Path::new(
            "src/audits/code_quality/complexity.rs"
        )));
        assert!(!is_test_file(Path::new("src/scan/cache.rs")));
    }
}
