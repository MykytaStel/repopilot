use crate::audits::traits::ProjectAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;

const TEST_FOLDER_NAMES: &[&str] = &["tests", "test", "__tests__", "spec"];

pub struct MissingTestFolderAudit;

impl ProjectAudit for MissingTestFolderAudit {
    fn audit(&self, facts: &ScanFacts, _config: &ScanConfig) -> Vec<Finding> {
        let has_test_folder = facts.files.iter().any(|file| {
            file.path.components().any(|c| {
                let name = c.as_os_str().to_string_lossy();
                TEST_FOLDER_NAMES.contains(&name.as_ref())
            })
        });

        if has_test_folder {
            return vec![];
        }

        vec![Finding {
            id: format!(
                "testing.missing-test-folder.{}",
                facts.root_path.display()
            ),
            rule_id: "testing.missing-test-folder".to_string(),
            title: "No test folder found".to_string(),
            description: format!(
                "No test directory (tests/, test/, __tests__/, spec/) was found under `{}`. Consider adding tests to improve confidence in refactoring.",
                facts.root_path.display()
            ),
            category: FindingCategory::Testing,
            severity: Severity::Medium,
            evidence: vec![Evidence {
                path: facts.root_path.clone(),
                line_start: 1,
                line_end: None,
                snippet: "No tests/, test/, __tests__/, or spec/ directory found.".to_string(),
            }],
        }]
    }
}
