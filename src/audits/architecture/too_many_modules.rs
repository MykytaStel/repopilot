use crate::audits::traits::ProjectAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct TooManyModulesAudit;

impl ProjectAudit for TooManyModulesAudit {
    fn audit(&self, facts: &ScanFacts, config: &ScanConfig) -> Vec<Finding> {
        let mut dir_file_counts: HashMap<PathBuf, usize> = HashMap::new();

        for file in &facts.files {
            if let Some(parent) = file.path.parent() {
                *dir_file_counts.entry(parent.to_path_buf()).or_insert(0) += 1;
            }
        }

        dir_file_counts
            .into_iter()
            .filter(|(_, count)| *count > config.max_directory_modules)
            .map(|(dir, count)| build_finding(dir, count, config.max_directory_modules))
            .collect()
    }
}

fn build_finding(dir: PathBuf, file_count: usize, threshold: usize) -> Finding {
    Finding {
        id: format!("architecture.too-many-modules.{}", dir.display()),
        rule_id: "architecture.too-many-modules".to_string(),
        title: "Directory contains too many modules".to_string(),
        description: format!(
            "This directory has {file_count} files, exceeding the threshold of {threshold}. Consider splitting into sub-modules to reduce coupling."
        ),
        category: FindingCategory::Architecture,
        severity: Severity::Medium,
        evidence: vec![Evidence {
            path: dir,
            line_start: 1,
            line_end: None,
            snippet: format!("{file_count} files in directory; threshold is {threshold}."),
        }],
    }
}
