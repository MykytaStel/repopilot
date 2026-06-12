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
            | "main.go"
            | "main.py"
            | "app.py"
            | "program.cs"
            | "main.java"
            | "main.kt"
            | "index.ts"
            | "index.js"
            | "main.ts"
            | "main.js"
    ) || (language == LanguageKind::Python && content.contains("if __name__ == \"__main__\""))
        || (language == LanguageKind::Go && content.contains("func main("))
        || (language == LanguageKind::Rust && content.contains("fn main("))
}

#[cfg(test)]
mod tests {
    use super::{is_test_file, path_contains_component};
    use std::path::Path;

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
