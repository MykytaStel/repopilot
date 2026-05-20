use crate::audits::traits::ProjectAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;
use classification::{is_low_signal_wrapper, is_source_file, is_test_file};
use matching::{has_nearby_test, tests_dir_suffix};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

mod classification;
mod matching;

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
        recommendation: Finding::recommendation_for_rule_id("testing.source-without-test"),
        title: "Source file has no corresponding test".to_string(),
        description: format!(
            "`{}` has no nearby test file. Consider adding tests to cover its behaviour.",
            source.display()
        ),
        category: FindingCategory::Testing,
        severity: Severity::Low,
        confidence: Default::default(),
        evidence: vec![Evidence {
            path: source.to_path_buf(),
            line_start: 1,
            line_end: None,
            snippet: format!("No test found; expected e.g. `{expected}`"),
        }],
        workspace_package: None,
        docs_url: None,
        provenance: Default::default(),
        risk: Default::default(),
    }
}
