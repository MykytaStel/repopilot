use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use std::path::Path;

pub const LARGE_FILE_LOC_THRESHOLD: usize = 300;
const HIGH_SEVERITY_LOC_THRESHOLD: usize = 1000;

pub fn detect_large_file_finding(path: &Path, lines_of_code: usize) -> Option<Finding> {
    if lines_of_code <= LARGE_FILE_LOC_THRESHOLD {
        return None;
    }

    Some(Finding {
        id: format!("architecture.large-file.{}", path.display()),
        rule_id: "architecture.large-file".to_string(),
        title: "Large file detected".to_string(),
        description: format!(
            "This file has {lines_of_code} non-empty lines of code, which is above the recommended threshold of {LARGE_FILE_LOC_THRESHOLD}. Consider splitting responsibilities into smaller modules."
        ),
        category: FindingCategory::Architecture,
        severity: severity_for_loc(lines_of_code),
        evidence: vec![Evidence {
            path: path.to_path_buf(),
            line_start: 1,
            line_end: None,
            snippet: format!(
                "File has {lines_of_code} non-empty lines of code; threshold is {LARGE_FILE_LOC_THRESHOLD}."
            ),
        }],
    })
}

fn severity_for_loc(lines_of_code: usize) -> Severity {
    if lines_of_code >= HIGH_SEVERITY_LOC_THRESHOLD {
        Severity::High
    } else {
        Severity::Medium
    }
}
