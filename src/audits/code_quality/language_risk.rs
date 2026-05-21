use crate::audits::code_quality::sanitize::{sanitize_c_style, sanitize_python_line};
use crate::audits::traits::FileAudit;
use crate::findings::types::Finding;
use crate::knowledge::language::language_id_for_name;
use crate::scan::config::ScanConfig;
use crate::scan::facts::FileFacts;
use std::path::Path;

pub struct LanguageRiskAudit;

impl FileAudit for LanguageRiskAudit {
    fn audit(&self, file: &FileFacts, _config: &ScanConfig) -> Vec<Finding> {
        let Some(content) = file
            .content
            .as_deref()
            .filter(|content| !content.is_empty())
        else {
            return vec![];
        };

        let Some(language_id) = file.language.as_deref().and_then(language_id_for_name) else {
            return vec![];
        };

        detect_language_risks(language_id, content, &file.path, file)
    }
}

fn detect_language_risks(
    language_id: &str,
    content: &str,
    path: &Path,
    file: &FileFacts,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut in_block_comment = false;

    for (line_index, raw_line) in content.lines().enumerate() {
        let sanitized = if language_id == "python" {
            match sanitize_python_line(raw_line) {
                Some(s) => s,
                None => continue,
            }
        } else {
            sanitize_c_style(raw_line, &mut in_block_comment)
        };

        let trimmed = sanitized.trim();
        if trimmed.is_empty() {
            continue;
        }

        emit_findings_for_line(
            language_id,
            trimmed,
            path,
            raw_line,
            line_index,
            file,
            &mut findings,
        );
    }

    findings
}

mod pattern;

use self::pattern::emit_findings_for_line;

#[cfg(test)]
mod tests;
