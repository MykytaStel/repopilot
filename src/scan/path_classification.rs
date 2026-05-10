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
}
