mod finding;
mod pattern;

#[cfg(test)]
mod tests;

use self::finding::build_finding;
use self::pattern::{
    detect_pattern, is_external_failure_path, is_infallible_render_write_result_unwrap,
    is_infallible_render_write_start, should_ignore_contextual_panic_pattern,
};
use crate::audits::code_quality::sanitize::sanitize_c_style;
use crate::audits::context::{LanguageKind, classify_file};
use crate::audits::traits::FileAudit;
use crate::findings::types::{Finding, Severity};
use crate::knowledge::decision::decide_for_audit_context;
use crate::scan::config::ScanConfig;
use crate::scan::facts::FileFacts;
use crate::scan::path_classification::is_low_signal_audit_path;

const RULE_ID: &str = "language.rust.panic-risk";

pub struct RustPanicRiskAudit;

impl FileAudit for RustPanicRiskAudit {
    fn audit(&self, file: &FileFacts, _config: &ScanConfig) -> Vec<Finding> {
        if is_low_signal_audit_path(&file.path) {
            return vec![];
        }

        let context = classify_file(file);

        if context.language != LanguageKind::Rust {
            return vec![];
        }

        let Some(content) = file.content.as_deref() else {
            return vec![];
        };

        let mut findings = Vec::new();
        let mut in_block_comment = false;
        let mut pending_render_write = false;

        for (index, line) in content.lines().enumerate() {
            let line_number = index + 1;
            let trimmed = line.trim();

            if trimmed.is_empty() {
                continue;
            }

            let sanitized = sanitize_c_style(line, &mut in_block_comment);
            let sanitized = sanitized.trim();
            if is_infallible_render_write_start(&file.path, sanitized) {
                pending_render_write = true;
            }

            let Some(pattern) = detect_pattern(sanitized) else {
                if sanitized.ends_with(';') {
                    pending_render_write = false;
                }
                continue;
            };

            if pending_render_write && is_infallible_render_write_result_unwrap(pattern, sanitized)
            {
                if sanitized.ends_with(';') {
                    pending_render_write = false;
                }
                continue;
            }

            if should_ignore_contextual_panic_pattern(pattern, trimmed) {
                if sanitized.ends_with(';') {
                    pending_render_write = false;
                }
                continue;
            }

            let decision = decide_for_audit_context(
                RULE_ID,
                &context,
                pattern.base_severity(),
                Some(pattern.signal()),
            );

            if decision.is_suppressed() {
                continue;
            }

            let severity = if is_external_failure_path(pattern, sanitized) && !context.is_test {
                decision.severity.max(Severity::High)
            } else {
                decision.severity
            };

            findings.push(build_finding(
                file,
                line_number,
                trimmed,
                pattern,
                &context,
                severity,
            ));

            if sanitized.ends_with(';') {
                pending_render_write = false;
            }
        }

        findings
    }
}
