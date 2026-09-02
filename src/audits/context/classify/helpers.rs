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

    // Directories that hold tests. The singular `test` is as common as the
    // plural — Node and Express use `test/`, Maven and Gradle use
    // `src/test/java`, Android adds `src/androidTest` — so recognizing only
    // `tests/` classified those whole trees as production code. Each name must
    // match a full path component, or `src/latest/` and `src/testing/` would be
    // swept in with them.
    const TEST_DIRECTORIES: &[&str] = &["tests", "test", "androidtest", "fixtures", "__tests__"];
    if has_test_directory(&path_text, TEST_DIRECTORIES)
        // Sibling test modules pulled in without a path component
        // (`tests_render.rs` and friends) — a cross-language prefix form.
        || file_name.starts_with("tests_")
        // A module whose entire name is `test`/`tests` is the file the runner
        // collects, not one production code imports — Django's generated app
        // puts its tests in a bare `tests.py`.
        || matches!(file_name.rsplit_once('.'), Some(("test" | "tests", _)))
    {
        return true;
    }

    // Language-specific file naming (`.spec.ts`, `_test.go`, plural
    // `_tests.rs`, …) comes from the frontend's conventions, as does whether
    // the `test_` prefix convention applies — it is NOT a Rust one (Rust
    // uses `tests/`, `#[cfg(test)]`, or plural `_tests.rs`), where it
    // collides with production modules like `test_edges.rs`.
    let conventions = crate::languages::conventions::conventions_for_path(path);
    (conventions.test_file_name)(&file_name)
        || (file_name.starts_with("test_") && conventions.test_prefix_marks_test)
}

fn has_test_directory(path_text: &str, names: &[&str]) -> bool {
    let mut components = path_text.split(['/', '\\']).collect::<Vec<_>>();
    components.pop(); // the file name itself is not a directory
    components.iter().any(|component| names.contains(component))
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
#[cfg(test)] // production callers go through the frontend conventions
pub fn is_test_support_file(path: &Path) -> bool {
    crate::languages::frontend_for_kind(LanguageKind::Rust)
        .conventions
        .test_support
        .is_some_and(|support| (support.matches)(path))
}

/// True for *build-tooling* sources — Gradle convention plugins and build logic
/// under `build-logic/` or `buildSrc/`. These configure the build and never ship
/// in the application, so a `throw`/`TODO()` there fails the build by design.
pub fn is_build_tooling_path(path: &Path) -> bool {
    path_contains_component(path, &["build-logic", "buildsrc"])
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
            // Third-party code checked into the repository. Nobody maintains it
            // here, so its size, complexity, and nesting are not this
            // repository's design decisions.
            "vendor",
            "vendors",
            "third_party",
        ],
    ) || is_minified(path)
        || content.contains("@generated")
        || content.contains("code generated")
        || content.contains("Code generated")
        || content.contains("Code Generated")
        || content.contains("generated by")
        || content.contains("Generated by")
        || content.contains("Generated By")
        || looks_like_vendored_bundle(content)
}

/// A minified artifact: `jquery-ui-1.13.2.min.js`. Machine-compressed output
/// where every maintainability metric measures the minifier, not an author.
fn is_minified(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            let lower = name.to_lowercase();
            lower.contains(".min.") || lower.contains("-min.")
        })
}

// Ignore generated JS/TS bundles identified by Emscripten markers or a bare
// whole-file lint opt-out; their runtime shims are not authored application code.
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
    ) || crate::languages::conventions::conventions_for_kind(language)
        .entrypoint_content
        .is_some_and(|probe| probe(content))
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

#[cfg(test)]
mod test_directory_tests {
    use super::is_test_file;
    use std::path::Path;

    #[test]
    fn singular_test_directories_are_recognized() {
        // Catches recognizing only `tests/`: Node, Maven, and Gradle all use
        // the singular form, so those trees classified as production code.
        for path in [
            "test/app.render.js",
            "test/acceptance/route-map.js",
            "src/test/java/org/example/EntityUtils.java",
            "core/ui/src/androidTest/kotlin/Foo.kt",
            "wagtail/test/utils/wagtail_tests.py",
        ] {
            assert!(is_test_file(Path::new(path)), "{path}");
        }
    }

    #[test]
    fn a_module_named_only_test_or_tests_is_a_test() {
        // Django's generated app puts its tests in a bare `tests.py`.
        assert!(is_test_file(Path::new("home/tests.py")));
        assert!(is_test_file(Path::new("home/test.py")));
        assert!(is_test_file(Path::new("pkg/tests.rs")));
    }

    #[test]
    fn a_directory_name_must_match_a_whole_component() {
        // The false-negative guard: production trees whose names merely
        // contain `test` must stay production.
        for path in [
            "src/testing/harness.ts",
            "src/latest/api.ts",
            "src/contest/rules.py",
            "src/test_helpers/build.ts",
            "protest/views.py",
        ] {
            assert!(!is_test_file(Path::new(path)), "{path}");
        }
    }

    #[test]
    fn a_file_whose_name_merely_starts_with_test_is_unchanged() {
        // `test_edges.rs` is production code in this repository; only the
        // frontend conventions decide whether a `test_` prefix marks a test.
        assert!(!is_test_file(Path::new("src/graph/test_edges.rs")));
        assert!(!is_test_file(Path::new("src/testament.ts")));
    }
}
