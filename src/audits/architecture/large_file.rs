use crate::analysis::{FileRole, classify_file_architecture};
use crate::audits::traits::FileAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::knowledge::decision::apply_file_decision;
use crate::scan::config::ScanConfig;
use crate::scan::facts::FileFacts;
use std::path::Path;

mod code_lines;

pub struct LargeFileAudit;

impl FileAudit for LargeFileAudit {
    fn audit(&self, file: &FileFacts, config: &ScanConfig) -> Vec<Finding> {
        let arch_ctx = classify_file_architecture(file, config);
        if arch_ctx.file_role != FileRole::Production {
            return vec![];
        }

        // Comments are documentation, not the size problem this rule reports.
        // The count falls back to every non-empty line when the language's
        // comment syntax is unknown or the content was not retained.
        let lines = file
            .content
            .as_deref()
            .and_then(|content| code_lines::count_code_lines(content, file.language.as_deref()))
            .unwrap_or(file.non_empty_lines);

        detect_large_file_finding(&file.path, lines, config)
            .and_then(|finding| apply_file_decision("architecture.large-file", file, finding, None))
            .into_iter()
            .collect()
    }
}

pub fn detect_large_file_finding(
    path: &Path,
    non_empty_lines: usize,
    config: &ScanConfig,
) -> Option<Finding> {
    if non_empty_lines <= config.large_file_loc_threshold {
        return None;
    }

    Some(Finding {
        id: String::new(),
        rule_id: "architecture.large-file".to_string(),
        recommendation: Finding::recommendation_for_rule_id("architecture.large-file"),
        title: "Large file detected".to_string(),
        description: format!(
            "This file has {non_empty_lines} non-empty lines of code, which is above the configured threshold of {}. Consider splitting responsibilities into smaller modules.",
            config.large_file_loc_threshold
        ),
        category: FindingCategory::Architecture,
        severity: severity_for_loc(non_empty_lines, config),
        confidence: Default::default(),
        evidence: vec![Evidence {
            path: path.to_path_buf(),
            line_start: 1,
            line_end: None,
            snippet: format!(
                "File has {non_empty_lines} non-empty lines of code; configured threshold is {}.",
                config.large_file_loc_threshold
            ),
        }],
        workspace_package: None,
        docs_url: None,
        provenance: Default::default(),
        risk: Default::default(),
    })
}

fn severity_for_loc(non_empty_lines: usize, config: &ScanConfig) -> Severity {
    if non_empty_lines >= config.huge_file_loc_threshold {
        Severity::High
    } else {
        Severity::Medium
    }
}
