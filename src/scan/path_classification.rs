use std::path::Path;

pub(crate) fn is_low_signal_audit_path(path: &Path) -> bool {
    has_low_signal_component(path) || has_test_like_file_name(path)
}

fn has_low_signal_component(path: &Path) -> bool {
    path.components().any(|component| {
        let name = component.as_os_str().to_string_lossy();
        matches!(
            name.as_ref(),
            "test"
                | "tests"
                | "__tests__"
                | "spec"
                | "specs"
                | "fixture"
                | "fixtures"
                | "mock"
                | "mocks"
                | "__mocks__"
                | "bench"
                | "benches"
                | "example"
                | "examples"
                | "generated"
                | "__generated__"
                | "gen"
                | "codegen"
        )
    })
}

fn has_test_like_file_name(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();

    // Rust test files use the `tests/` directory (caught by the component check),
    // `#[cfg(test)]`, or the plural `tests.rs` / `*_tests.rs` names. The singular
    // `test_*` / `*_test` patterns are Python/Go/JS conventions that collide with
    // ordinary Rust production modules (`test_edges.rs`, `source_without_test.rs`),
    // matching the refined `is_test_file`. Applying them here silently dropped
    // those production files from the scan — and thus from every audit and the
    // import graph — so for `.rs` files only the plural Rust forms qualify.
    if name.ends_with(".rs") {
        return name == "tests.rs" || name.ends_with("_tests.rs");
    }

    stem == "test"
        || stem == "tests"
        || stem.ends_with("_test")
        || stem.ends_with(".test")
        || stem.ends_with(".spec")
        || name.starts_with("test_")
        || name.contains(".test.")
        || name.contains(".spec.")
}

#[cfg(test)]
mod tests {
    use super::is_low_signal_audit_path;
    use std::path::Path;

    #[test]
    fn detects_conventional_test_and_fixture_paths() {
        for path in [
            "tests/scan.rs",
            "src/foo.spec.ts",
            "src/__tests__/user.ts",
            "src/fixtures/data.rs",
            "examples/demo.rs",
            "benches/parser.rs",
        ] {
            assert!(
                is_low_signal_audit_path(Path::new(path)),
                "{path} should be treated as low-signal audit path"
            );
        }
    }

    #[test]
    fn production_source_paths_are_not_low_signal() {
        for path in ["src/scanner.rs", "src/audit/large_file.rs", "app/main.ts"] {
            assert!(
                !is_low_signal_audit_path(Path::new(path)),
                "{path} should remain eligible for medium audits"
            );
        }
    }

    #[test]
    fn rust_singular_test_name_patterns_are_production() {
        // `test_*` / `*_test` are not Rust test conventions; these are real
        // production modules that must still be scanned and audited.
        for path in [
            "src/graph/test_edges.rs",
            "src/audits/testing/source_without_test.rs",
        ] {
            assert!(
                !is_low_signal_audit_path(Path::new(path)),
                "{path} is a Rust production module and must not be low-signal"
            );
        }
    }

    #[test]
    fn rust_plural_test_modules_remain_low_signal() {
        for path in [
            "src/audits/foo/tests.rs",
            "src/audits/code_quality/behavioral_tests.rs",
        ] {
            assert!(
                is_low_signal_audit_path(Path::new(path)),
                "{path} is a Rust test module and should stay low-signal"
            );
        }
    }

    #[test]
    fn non_rust_singular_test_names_remain_low_signal() {
        for path in ["pkg/user_test.go", "app/test_helpers.py"] {
            assert!(
                is_low_signal_audit_path(Path::new(path)),
                "{path} follows a non-Rust test convention and should stay low-signal"
            );
        }
    }
}
