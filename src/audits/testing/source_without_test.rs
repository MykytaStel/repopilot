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
        let all_paths: HashSet<PathBuf> = facts.files.iter().map(|f| f.path.clone()).collect();

        facts
            .files
            .iter()
            .filter(|file| is_source_file(&file.path))
            .filter(|file| !is_test_file(&file.path))
            .filter(|file| !has_nearby_test(&file.path, &all_paths))
            .map(|file| build_finding(&file.path))
            .collect()
    }
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

fn has_nearby_test(source: &Path, all_paths: &HashSet<PathBuf>) -> bool {
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
    let tests_candidates = [
        PathBuf::from("tests").join(format!("{stem}.{ext}")),
        PathBuf::from("tests").join(format!("{stem}_test.{ext}")),
    ];

    // Check if any existing path ends with these relative paths
    tests_candidates.iter().any(|candidate| {
        all_paths
            .iter()
            .any(|p| p.ends_with(candidate.as_path()))
    })
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
        id: format!("testing.source-without-test.{}", source.display()),
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
