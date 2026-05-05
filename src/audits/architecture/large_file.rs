use crate::audits::traits::FileAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::scan::config::ScanConfig;
use crate::scan::facts::FileFacts;
use std::path::Path;

pub struct LargeFileAudit;

impl FileAudit for LargeFileAudit {
    fn audit(&self, file: &FileFacts, config: &ScanConfig) -> Vec<Finding> {
        detect_large_file_finding(&file.path, file.lines_of_code, config)
            .into_iter()
            .collect()
    }
}

pub fn detect_large_file_finding(
    path: &Path,
    lines_of_code: usize,
    config: &ScanConfig,
) -> Option<Finding> {
    if lines_of_code <= config.large_file_loc_threshold {
        return None;
    }

    Some(Finding {
        id: format!("architecture.large-file.{}", path.display()),
        rule_id: "architecture.large-file".to_string(),
        title: "Large file detected".to_string(),
        description: format!(
            "This file has {lines_of_code} non-empty lines of code, which is above the configured threshold of {}. Consider splitting responsibilities into smaller modules.",
            config.large_file_loc_threshold
        ),
        category: FindingCategory::Architecture,
        severity: severity_for_loc(lines_of_code, config),
        evidence: vec![Evidence {
            path: path.to_path_buf(),
            line_start: 1,
            line_end: None,
            snippet: format!(
                "File has {lines_of_code} non-empty lines of code; configured threshold is {}.",
                config.large_file_loc_threshold
            ),
        }],
    })
}

fn severity_for_loc(lines_of_code: usize, config: &ScanConfig) -> Severity {
    if lines_of_code >= config.huge_file_loc_threshold {
        Severity::High
    } else {
        Severity::Medium
    }
}
