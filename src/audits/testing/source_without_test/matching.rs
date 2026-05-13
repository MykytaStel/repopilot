use std::collections::HashSet;
use std::path::{Path, PathBuf};

const TEST_EXTENSIONS: &[&str] = &["rs", "ts", "tsx", "js", "jsx", "py", "go", "java", "kt"];

/// Returns the path suffix starting at the `tests/` or `__tests__/` component, normalised to forward slashes.
pub(super) fn tests_dir_suffix(path: &Path) -> Option<String> {
    let components: Vec<_> = path.components().collect();
    let idx = components.iter().position(|c| {
        let s = c.as_os_str().to_string_lossy();
        s == "tests" || s == "__tests__"
    })?;
    let suffix: PathBuf = components[idx..].iter().collect();
    Some(suffix.to_string_lossy().replace('\\', "/"))
}

pub(super) fn has_nearby_test(
    source: &Path,
    all_paths: &HashSet<PathBuf>,
    tests_suffixes: &HashSet<String>,
) -> bool {
    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();

    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();

    if !TEST_EXTENSIONS.contains(&ext) {
        return true;
    }

    let parent = source.parent().unwrap_or(Path::new("."));

    has_sibling_test(parent, stem, ext, all_paths)
        || has_tests_directory_match(stem, ext, tests_suffixes)
        || has_rust_integration_test(source, ext, tests_suffixes)
}

fn has_sibling_test(parent: &Path, stem: &str, ext: &str, all_paths: &HashSet<PathBuf>) -> bool {
    [
        parent.join(format!("{stem}_test.{ext}")),
        parent.join(format!("{stem}.test.{ext}")),
        parent.join(format!("{stem}.spec.{ext}")),
    ]
    .iter()
    .any(|candidate| all_paths.contains(candidate))
}

fn has_tests_directory_match(stem: &str, ext: &str, tests_suffixes: &HashSet<String>) -> bool {
    const DIRS: &[&str] = &["tests", "__tests__"];
    const SUFFIXES: &[&str] = &["", "_test", ".test", ".spec"];
    DIRS.iter()
        .flat_map(|dir| {
            SUFFIXES
                .iter()
                .map(move |suf| format!("{dir}/{stem}{suf}.{ext}"))
        })
        .any(|candidate| tests_suffixes.contains(&candidate))
}

fn has_rust_integration_test(source: &Path, ext: &str, tests_suffixes: &HashSet<String>) -> bool {
    if ext != "rs" {
        return false;
    }

    let module_candidate = format!("tests/{}.rs", module_test_name(source));
    if tests_suffixes.contains(module_candidate.as_str()) {
        return true;
    }

    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();

    let module_parts = rust_module_parts(source);
    let stem_parts: Vec<&str> = stem
        .split('_')
        .chain(module_parts.iter().map(String::as_str))
        .filter(|p| p.len() >= 4)
        .collect();

    if stem_parts.is_empty() {
        return false;
    }

    tests_suffixes.iter().any(|suffix| {
        let test_name = Path::new(suffix)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        stem_parts.iter().any(|part| test_name.contains(part))
    })
}

fn module_test_name(source: &Path) -> String {
    let Some(src_index) = source
        .components()
        .position(|c| c.as_os_str().to_string_lossy() == "src")
    else {
        return source
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
    };

    source
        .components()
        .skip(src_index + 1)
        .filter_map(|c| {
            let value = c.as_os_str().to_string_lossy();
            let value = value.strip_suffix(".rs").unwrap_or(value.as_ref());
            (value != "mod").then(|| value.to_string())
        })
        .collect::<Vec<_>>()
        .join("_")
}

fn rust_module_parts(source: &Path) -> Vec<String> {
    let Some(src_index) = source
        .components()
        .position(|c| c.as_os_str().to_string_lossy() == "src")
    else {
        return Vec::new();
    };

    source
        .components()
        .skip(src_index + 1)
        .filter_map(|c| {
            let value = c.as_os_str().to_string_lossy();
            let value = value.strip_suffix(".rs").unwrap_or(value.as_ref());
            (!matches!(value, "mod" | "lib" | "main")).then(|| value.to_string())
        })
        .collect()
}
