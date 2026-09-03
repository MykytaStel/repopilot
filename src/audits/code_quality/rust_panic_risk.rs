mod ast;
mod finding;
mod pattern;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_render;

use crate::analysis::parse::ParsedFile;
use crate::audits::code_quality::sanitize::sanitize_c_style;
use crate::audits::context::{LanguageKind, classify_file};
use crate::audits::traits::FileAudit;
use crate::findings::provenance::{AnalysisScope, FindingProvenance};
use crate::findings::types::{Finding, Severity};
use crate::knowledge::decision::{decide_for_audit_context, record_decision_provenance};
use crate::rules::{RuleLifecycle, SignalSource, lookup_rule_metadata};
use crate::scan::config::ScanConfig;
use crate::scan::facts::FileFacts;
use crate::scan::path_classification::is_low_signal_audit_path;

use self::finding::build_finding;
use self::pattern::{
    detect_pattern, is_external_failure_path, is_infallible_literal_construction_unwrap,
    is_infallible_render_write_result_unwrap, is_infallible_render_write_start,
    is_literal_parse_unwrap, is_literal_parse_unwrap_line, is_report_renderer_path,
    is_structural_infallible_render_write_unwrap, should_ignore_contextual_panic_pattern,
};

const RULE_ID: &str = "language.rust.panic-risk";

pub struct RustPanicRiskAudit;

impl FileAudit for RustPanicRiskAudit {
    fn audit(&self, file: &FileFacts, config: &ScanConfig) -> Vec<Finding> {
        self.analyze(file, &ParsedFile::for_facts(file), config)
    }

    fn audit_parsed(
        &self,
        file: &FileFacts,
        parsed: &ParsedFile,
        config: &ScanConfig,
    ) -> Vec<Finding> {
        self.analyze(file, parsed, config)
    }
}

impl RustPanicRiskAudit {
    fn analyze(&self, file: &FileFacts, parsed: &ParsedFile, _config: &ScanConfig) -> Vec<Finding> {
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

        match parsed.tree() {
            Some(tree) => self.analyze_ast_candidates(file, content, &context, tree.root_node()),
            None => {
                let mut findings = self.line_scan(file, content, &context);
                mark_text_heuristic(&mut findings);
                findings
            }
        }
    }

    fn analyze_ast_candidates(
        &self,
        file: &FileFacts,
        content: &str,
        context: &crate::audits::context::AuditContext,
        root: tree_sitter::Node<'_>,
    ) -> Vec<Finding> {
        let candidates = ast::collect_candidates(root, content);
        let mut sorted_lines: Vec<usize> = candidates.keys().copied().collect();
        sorted_lines.sort_unstable();
        sorted_lines
            .into_iter()
            .filter_map(|line_number| {
                let (node, pattern) = candidates.get(&line_number)?;
                self.analyze_candidate(file, content, context, line_number, *node, *pattern)
            })
            .collect()
    }

    fn analyze_candidate(
        &self,
        file: &FileFacts,
        content: &str,
        context: &crate::audits::context::AuditContext,
        line_number: usize,
        node: tree_sitter::Node<'_>,
        pattern: pattern::RustPanicPattern,
    ) -> Option<Finding> {
        let trimmed_line = content.lines().nth(line_number - 1).unwrap_or("").trim();
        if should_skip_ast_candidate(file, content, node, pattern, trimmed_line) {
            return None;
        }

        let decision = decide_for_audit_context(
            RULE_ID,
            context,
            pattern.base_severity(),
            Some(pattern.signal()),
        );
        if decision.is_suppressed() {
            return None;
        }

        let mut in_block_comment = false;
        let sanitized = sanitize_c_style(trimmed_line, &mut in_block_comment);
        let severity = candidate_severity(
            node,
            content,
            trimmed_line,
            pattern,
            sanitized.trim(),
            context,
            decision.severity,
        );
        let mut finding =
            build_finding(file, line_number, trimmed_line, pattern, context, severity);
        record_decision_provenance(
            &mut finding,
            pattern.base_severity(),
            Some(pattern.signal()),
            &decision,
        );
        Some(finding)
    }

    fn line_scan(
        &self,
        file: &FileFacts,
        content: &str,
        context: &crate::audits::context::AuditContext,
    ) -> Vec<Finding> {
        let mut state = LineScanState::default();
        content
            .lines()
            .enumerate()
            .filter_map(|(index, line)| scan_line(file, line, index + 1, context, &mut state))
            .collect()
    }
}

#[derive(Default)]
struct LineScanState {
    in_block_comment: bool,
    pending_render_write: bool,
}

fn should_skip_ast_candidate(
    file: &FileFacts,
    content: &str,
    node: tree_sitter::Node<'_>,
    pattern: pattern::RustPanicPattern,
    trimmed_line: &str,
) -> bool {
    (is_report_renderer_path(&file.path)
        && is_structural_infallible_render_write_unwrap(node, content))
        || is_infallible_literal_construction_unwrap(node, content)
        || should_ignore_contextual_panic_pattern(pattern, trimmed_line)
}

fn candidate_severity(
    node: tree_sitter::Node<'_>,
    content: &str,
    trimmed_line: &str,
    pattern: pattern::RustPanicPattern,
    sanitized: &str,
    context: &crate::audits::context::AuditContext,
    decision_severity: Severity,
) -> Severity {
    if is_literal_parse_unwrap(node, content) || is_literal_parse_unwrap_line(trimmed_line) {
        Severity::Low
    } else if is_external_failure_path(pattern, sanitized) && !context.is_test {
        decision_severity.max(Severity::High)
    } else {
        decision_severity
    }
}

fn scan_line(
    file: &FileFacts,
    line: &str,
    line_number: usize,
    context: &crate::audits::context::AuditContext,
    state: &mut LineScanState,
) -> Option<Finding> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let sanitized = sanitize_c_style(line, &mut state.in_block_comment);
    let sanitized = sanitized.trim();
    if is_infallible_render_write_start(&file.path, sanitized) {
        state.pending_render_write = true;
    }

    let Some(pattern) = detect_pattern(sanitized) else {
        clear_pending_render_write(sanitized, state);
        return None;
    };
    if skip_line_pattern(pattern, sanitized, trimmed, state) {
        return None;
    }

    let decision = decide_for_audit_context(
        RULE_ID,
        context,
        pattern.base_severity(),
        Some(pattern.signal()),
    );
    if decision.is_suppressed() {
        return None;
    }

    let severity = if is_external_failure_path(pattern, sanitized) && !context.is_test {
        decision.severity.max(Severity::High)
    } else {
        decision.severity
    };
    let mut finding = build_finding(file, line_number, trimmed, pattern, context, severity);
    record_decision_provenance(
        &mut finding,
        pattern.base_severity(),
        Some(pattern.signal()),
        &decision,
    );
    clear_pending_render_write(sanitized, state);
    Some(finding)
}

fn skip_line_pattern(
    pattern: pattern::RustPanicPattern,
    sanitized: &str,
    trimmed: &str,
    state: &mut LineScanState,
) -> bool {
    let skip = (state.pending_render_write
        && is_infallible_render_write_result_unwrap(pattern, sanitized))
        || should_ignore_contextual_panic_pattern(pattern, trimmed);
    if skip {
        clear_pending_render_write(sanitized, state);
    }
    skip
}

fn clear_pending_render_write(sanitized: &str, state: &mut LineScanState) {
    if sanitized.ends_with(';') {
        state.pending_render_write = false;
    }
}

fn mark_text_heuristic(findings: &mut [Finding]) {
    for finding in findings {
        let lifecycle = lookup_rule_metadata(&finding.rule_id)
            .map(|metadata| metadata.lifecycle)
            .unwrap_or(RuleLifecycle::Preview);
        let knowledge_decision = finding.provenance.knowledge_decision.take();
        finding.provenance = FindingProvenance {
            detector: finding.rule_id.clone(),
            signal_source: SignalSource::TextHeuristic,
            rule_lifecycle: lifecycle,
            analysis_scope: AnalysisScope::File,
            knowledge_decision,
        };
    }
}
