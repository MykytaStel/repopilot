use crate::audits::traits::ProjectAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

const SOURCE_EXTENSIONS: &[&str] = &["rs", "ts", "tsx", "js", "jsx", "py", "go", "java", "kt"];
const TEST_EXTENSIONS: &[&str] = &["rs", "ts", "tsx", "js", "jsx", "py", "go", "java", "kt"];

pub struct SourceWithoutTestAudit;

impl ProjectAudit for SourceWithoutTestAudit {
    fn audit(&self, facts: &ScanFacts, _config: &ScanConfig) -> Vec<Finding> {
        // Single pass: collect all paths + extract tests/ suffix set simultaneously
        let mut all_paths: HashSet<PathBuf> = HashSet::with_capacity(facts.files.len());
        let mut tests_suffixes: HashSet<String> = HashSet::new();
        for f in &facts.files {
            if let Some(suffix) = tests_dir_suffix(&f.path) {
                tests_suffixes.insert(suffix);
            }
            all_paths.insert(f.path.clone());
        }

        facts
            .files
            .iter()
            .filter(|file| is_source_file(&file.path))
            .filter(|file| !is_test_file(&file.path))
            .filter(|file| !is_low_signal_wrapper(&file.path))
            .filter(|file| !file.has_inline_tests)
            .filter(|file| !has_nearby_test(&file.path, &all_paths, &tests_suffixes))
            .map(|file| build_finding(&file.path))
            .collect()
    }
}

/// Returns the path suffix starting at the `tests/` component, normalised to forward slashes.
fn tests_dir_suffix(path: &Path) -> Option<String> {
    let components: Vec<_> = path.components().collect();
    let idx = components
        .iter()
        .position(|c| c.as_os_str().to_string_lossy() == "tests")?;
    let suffix: PathBuf = components[idx..].iter().collect();
    Some(suffix.to_string_lossy().replace('\\', "/"))
}

fn is_source_file(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();
    SOURCE_EXTENSIONS.contains(&ext) && !is_declaration_file(path) && !is_excluded_directory(path)
}

/// TypeScript/JavaScript declaration files contain only type annotations — no executable code.
/// `path.extension()` returns "ts" for "foo.d.ts", so we must check the full file name.
fn is_declaration_file(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    name.ends_with(".d.ts") || name.ends_with(".d.mts") || name.ends_with(".d.cts")
}

fn is_excluded_directory(path: &Path) -> bool {
    path.components().any(|c| {
        let name = c.as_os_str().to_string_lossy();
        matches!(
            name.as_ref(),
            // Test infrastructure
            "tests" | "test" | "__tests__" | "spec" | "fixtures"
            // Utility / entry-point directories
            | "bin" | "scripts" | "script" | "tools" | "tool"
            | "examples" | "example"
            // Type definition directories — never contain testable logic
            | "types" | "@types"
            // Auto-generated code
            | "generated" | "__generated__" | "gen" | "codegen"
            // Mock definitions used by tests (not tested themselves)
            | "mocks" | "__mocks__"
            // Static assets
            | "assets" | "public"
            // DB migrations
            | "migrations"
        )
    })
}

fn is_test_file(path: &Path) -> bool {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    stem == "tests"
        || stem == "test"
        || stem.ends_with("_test")
        || stem.ends_with(".test")
        || stem.ends_with(".spec")
}

fn is_low_signal_wrapper(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default();

    // Rust entry points and build-time infrastructure
    if matches!(name, "mod.rs" | "lib.rs" | "main.rs" | "build.rs") {
        return true;
    }

    // Rust stub/mock helper files (never contain testable production logic)
    if name.ends_with("stub.rs") || name.ends_with("mock.rs") || name.ends_with("fakes.rs") {
        return true;
    }

    // Universal barrel files (re-export only, no logic)
    if matches!(
        name,
        "index.ts"
            | "index.tsx"
            | "index.js"
            | "index.jsx"
            | "main.ts"
            | "main.tsx"
            | "main.js"
            | "main.jsx"
            | "__init__.py"
    ) {
        return true;
    }

    // TypeScript/JavaScript: well-known constant / type-only filenames
    if matches!(
        name,
        "types.ts"
            | "types.js"
            | "constants.ts"
            | "constants.js"
            | "tokens.ts"
            | "tokens.js"
            | "theme.ts"
            | "theme.js"
            | "colors.ts"
            | "colors.js"
            | "enums.ts"
            | "enums.js"
            | "globals.ts"
            | "globals.js"
    ) {
        return true;
    }

    // TypeScript/JavaScript compound-suffix patterns (e.g. user.types.ts, api.config.ts)
    if name.ends_with(".types.ts")
        || name.ends_with(".type.ts")
        || name.ends_with(".config.ts")
        || name.ends_with(".config.tsx")
        || name.ends_with(".config.js")
        || name.ends_with(".config.jsx")
        || name.ends_with(".config.mjs")
        || name.ends_with(".config.cjs")
        || name.ends_with(".constants.ts")
        || name.ends_with(".tokens.ts")
        || name.ends_with(".d.ts")
    {
        return true;
    }

    // Python: infrastructure files that are not unit-tested themselves
    if matches!(
        name,
        "setup.py" | "settings.py" | "conftest.py" | "__main__.py"
    ) {
        return true;
    }

    false
}

fn has_nearby_test(
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
        return true; // Non-standard extension — don't flag
    }

    let parent = source.parent().unwrap_or(Path::new("."));

    if has_sibling_test(parent, stem, ext, all_paths)
        || has_tests_directory_match(stem, ext, tests_suffixes)
        || has_rust_integration_test(source, ext, tests_suffixes)
    {
        return true;
    }

    false
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
    [
        format!("tests/{stem}.{ext}"),
        format!("tests/{stem}_test.{ext}"),
    ]
    .iter()
    .any(|candidate| tests_suffixes.contains(candidate.as_str()))
}

fn has_rust_integration_test(source: &Path, ext: &str, tests_suffixes: &HashSet<String>) -> bool {
    if ext != "rs" {
        return false;
    }

    // 1. Exact module-path match (e.g. tests/audits_security_secret_candidate.rs)
    let module_candidate = format!("tests/{}.rs", module_test_name(source));
    if tests_suffixes.contains(module_candidate.as_str()) {
        return true;
    }

    // 2. Fuzzy stem match: handles test files named after features rather than modules,
    //    e.g. `import_coupling.rs` is covered by `coupling_audit.rs` because "coupling"
    //    appears in both. Requires a minimum part length (≥5) to avoid false positives
    //    on short common words like "file" or "base".
    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();

    let stem_parts: Vec<&str> = stem.split('_').filter(|p| p.len() >= 5).collect();

    if stem_parts.is_empty() {
        return false;
    }

    tests_suffixes.iter().any(|suffix| {
        let test_name = std::path::Path::new(suffix)
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

fn build_finding(source: &Path) -> Finding {
    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();

    let expected = format!("{stem}_test.{ext}");

    Finding {
        id: String::new(),
        rule_id: "testing.source-without-test".to_string(),
        title: "Source file has no corresponding test".to_string(),
        description: format!(
            "`{}` has no nearby test file. Consider adding tests to cover its behaviour.",
            source.display()
        ),
        category: FindingCategory::Testing,
        severity: Severity::Low,
        evidence: vec![Evidence {
            path: source.to_path_buf(),
            line_start: 1,
            line_end: None,
            snippet: format!("No test found; expected e.g. `{expected}`"),
        }],
        workspace_package: None,
        docs_url: None,
    }
}
