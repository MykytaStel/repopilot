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
    if !SOURCE_EXTENSIONS.contains(&ext) {
        return false;
    }
    // Skip files already inside a test folder
    !path.components().any(|c| {
        let name = c.as_os_str().to_string_lossy();
        matches!(
            name.as_ref(),
            "tests" | "test" | "__tests__" | "spec" | "fixtures"
        )
    })
}

fn is_test_file(path: &Path) -> bool {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    stem.ends_with("_test") || stem.ends_with(".test") || stem.ends_with(".spec")
}

fn is_low_signal_wrapper(path: &Path) -> bool {
    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default();

    matches!(file_name, "mod.rs" | "lib.rs" | "main.rs")
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

    // Sibling test patterns: payment_test.rs, payment.test.ts, payment.spec.ts
    let sibling_candidates = [
        parent.join(format!("{stem}_test.{ext}")),
        parent.join(format!("{stem}.test.{ext}")),
        parent.join(format!("{stem}.spec.{ext}")),
    ];

    if sibling_candidates.iter().any(|p| all_paths.contains(p)) {
        return true;
    }

    // tests/ directory alongside src/: tests/<stem>.rs, tests/<stem>_test.rs
    // Uses pre-computed suffix set for O(1) lookup instead of O(n) scan
    let tests_candidates = [
        format!("tests/{stem}.{ext}"),
        format!("tests/{stem}_test.{ext}"),
    ];

    if tests_candidates
        .iter()
        .any(|candidate| tests_suffixes.contains(candidate.as_str()))
    {
        return true;
    }

    // Rust integration tests commonly cover a module by feature name:
    // src/report/writer.rs -> tests/report_writer.rs
    if ext == "rs" {
        let module_candidate = format!("tests/{}.rs", module_test_name(source));
        return tests_suffixes.contains(module_candidate.as_str());
    }

    false
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
    }
}
