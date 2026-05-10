use crate::audits::traits::FileAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::scan::config::ScanConfig;
use crate::scan::facts::FileFacts;
use crate::scan::path_classification::is_low_signal_audit_path;
use std::path::Path;

pub struct LargeFileAudit;

impl FileAudit for LargeFileAudit {
    fn audit(&self, file: &FileFacts, config: &ScanConfig) -> Vec<Finding> {
        if is_low_signal_audit_path(&file.path) {
            return vec![];
        }
        if !is_code_file(file.language.as_deref()) {
            return vec![];
        }
        detect_large_file_finding(&file.path, file.lines_of_code, config)
            .into_iter()
            .collect()
    }
}

/// Only applies the size limit to actual programming-language source files.
/// Documentation, config, and data formats (Markdown, YAML, JSON, TOML…)
/// are not subject to this rule because their length is driven by content, not code complexity.
fn is_code_file(language: Option<&str>) -> bool {
    matches!(
        language,
        Some(
            "Rust"
                | "Go"
                | "Python"
                | "TypeScript"
                | "TypeScript React"
                | "JavaScript"
                | "JavaScript React"
                | "Java"
                | "Kotlin"
                | "Swift"
                | "C#"
                | "C++"
                | "C"
                | "C/C++ Header"
        )
    )
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
        id: String::new(),
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
        workspace_package: None,
        docs_url: None,
    })
}

fn severity_for_loc(lines_of_code: usize, config: &ScanConfig) -> Severity {
    if lines_of_code >= config.huge_file_loc_threshold {
        Severity::High
    } else {
        Severity::Medium
    }
}
