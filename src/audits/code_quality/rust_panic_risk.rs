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
            Some(tree) => {
                let candidates = ast::collect_candidates(tree.root_node(), content);

                let mut findings = Vec::new();
                let mut sorted_lines: Vec<usize> = candidates.keys().copied().collect();
                sorted_lines.sort_unstable();

                for line_number in sorted_lines {
                    let (node, pattern) = candidates.get(&line_number).unwrap();
                    let trimmed_line = content.lines().nth(line_number - 1).unwrap_or("").trim();

                    // Apply infallible write unwrap structural check
                    if is_report_renderer_path(&file.path)
                        && is_structural_infallible_render_write_unwrap(*node, content)
                    {
                        continue;
                    }

                    // A literal `Regex::new("…")`/`Selector::parse("…")` only fails
                    // on a malformed compile-time pattern — a deterministic bug, not
                    // a runtime panic risk — so skip it structurally (handles the
                    // multi-line form the text heuristic below cannot).
                    if is_infallible_literal_construction_unwrap(*node, content) {
                        continue;
                    }

                    // Apply contextual ignore patterns
                    if should_ignore_contextual_panic_pattern(*pattern, trimmed_line) {
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

                    // Sanitize line for external failure checks
                    let mut in_block_comment = false;
                    let sanitized = sanitize_c_style(trimmed_line, &mut in_block_comment);
                    let sanitized = sanitized.trim();

                    // A `"literal".parse().unwrap()` parses authored input: it can
                    // still panic (`"999".parse::<u8>()`), but as a deterministic bug
                    // caught on the first run, not external-input risk. Downgrade it
                    // to Low (hidden in default, kept in strict) rather than escalate.
                    // The text fallback handles the macro form (`vec![…parse()…]`)
                    // whose body is an unparsed token tree the AST check cannot see.
                    let severity = if is_literal_parse_unwrap(*node, content)
                        || is_literal_parse_unwrap_line(trimmed_line)
                    {
                        Severity::Low
                    } else if is_external_failure_path(*pattern, sanitized) && !context.is_test {
                        decision.severity.max(Severity::High)
                    } else {
                        decision.severity
                    };

                    let mut finding = build_finding(
                        file,
                        line_number,
                        trimmed_line,
                        *pattern,
                        &context,
                        severity,
                    );
                    record_decision_provenance(
                        &mut finding,
                        pattern.base_severity(),
                        Some(pattern.signal()),
                        &decision,
                    );
                    findings.push(finding);
                }

                findings
            }
            None => {
                let mut findings = self.line_scan(file, content, &context);
                mark_text_heuristic(&mut findings);
                findings
            }
        }
    }

    fn line_scan(
        &self,
        file: &FileFacts,
        content: &str,
        context: &crate::audits::context::AuditContext,
    ) -> Vec<Finding> {
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
                context,
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

            let mut finding = build_finding(file, line_number, trimmed, pattern, context, severity);
            record_decision_provenance(
                &mut finding,
                pattern.base_severity(),
                Some(pattern.signal()),
                &decision,
            );
            findings.push(finding);

            if sanitized.ends_with(';') {
                pending_render_write = false;
            }
        }

        findings
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
