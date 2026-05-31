mod finding;
mod pattern;

#[cfg(test)]
mod tests;

use crate::analysis::parse::ParsedFile;
use crate::audits::code_quality::sanitize::sanitize_c_style;
use crate::audits::context::{LanguageKind, classify_file};
use crate::audits::traits::FileAudit;
use crate::findings::provenance::{AnalysisScope, FindingProvenance};
use crate::findings::types::{Finding, Severity};
use crate::knowledge::decision::decide_for_audit_context;
use crate::rules::{RuleLifecycle, SignalSource, lookup_rule_metadata};
use crate::scan::config::ScanConfig;
use crate::scan::facts::FileFacts;
use crate::scan::path_classification::is_low_signal_audit_path;
use std::collections::HashMap;
use tree_sitter::Node;

use self::finding::build_finding;
use self::pattern::{
    RustPanicPattern, detect_pattern, is_external_failure_path,
    is_infallible_render_write_result_unwrap, is_infallible_render_write_start,
    is_report_renderer_path, is_structural_infallible_render_write_unwrap,
    should_ignore_contextual_panic_pattern,
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
                let mut candidates: HashMap<usize, (Node<'_>, RustPanicPattern)> = HashMap::new();

                fn visit<'a>(
                    node: Node<'a>,
                    content: &str,
                    candidates: &mut HashMap<usize, (Node<'a>, RustPanicPattern)>,
                ) {
                    if let Some(pattern) = detect_ast_node(node, content) {
                        let line_number = node.start_position().row + 1;
                        let keep = if let Some((_, existing_pattern)) = candidates.get(&line_number)
                        {
                            pattern.precedence() > existing_pattern.precedence()
                        } else {
                            true
                        };
                        if keep {
                            candidates.insert(line_number, (node, pattern));
                        }
                    }

                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        visit(child, content, candidates);
                    }
                }

                visit(tree.root_node(), content, &mut candidates);

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

                    let severity =
                        if is_external_failure_path(*pattern, sanitized) && !context.is_test {
                            decision.severity.max(Severity::High)
                        } else {
                            decision.severity
                        };

                    findings.push(build_finding(
                        file,
                        line_number,
                        trimmed_line,
                        *pattern,
                        &context,
                        severity,
                    ));
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

            findings.push(build_finding(
                file,
                line_number,
                trimmed,
                pattern,
                context,
                severity,
            ));

            if sanitized.ends_with(';') {
                pending_render_write = false;
            }
        }

        findings
    }
}

fn detect_ast_node(node: Node<'_>, content: &str) -> Option<RustPanicPattern> {
    match node.kind() {
        "macro_invocation" => {
            let macro_node = node
                .child_by_field_name("macro")
                .or_else(|| node.child(0))?;
            let text = macro_node.utf8_text(content.as_bytes()).ok()?;
            let macro_name = text.split("::").last()?;
            match macro_name {
                "panic" => Some(RustPanicPattern::Panic),
                "todo" => Some(RustPanicPattern::Todo),
                "unimplemented" => Some(RustPanicPattern::Unimplemented),
                _ => None,
            }
        }
        "call_expression" => {
            let function = node.child_by_field_name("function")?;
            if function.kind() == "field_expression" {
                let field = function.child_by_field_name("field")?;
                let method_name = field.utf8_text(content.as_bytes()).ok()?;
                match method_name {
                    "unwrap" => Some(RustPanicPattern::Unwrap),
                    "unwrap_err" => Some(RustPanicPattern::UnwrapErr),
                    "expect" => Some(RustPanicPattern::Expect),
                    "expect_err" => Some(RustPanicPattern::ExpectErr),
                    _ => None,
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

fn mark_text_heuristic(findings: &mut [Finding]) {
    for finding in findings {
        let lifecycle = lookup_rule_metadata(&finding.rule_id)
            .map(|metadata| metadata.lifecycle)
            .unwrap_or(RuleLifecycle::Preview);
        finding.provenance = FindingProvenance {
            detector: finding.rule_id.clone(),
            signal_source: SignalSource::TextHeuristic,
            rule_lifecycle: lifecycle,
            analysis_scope: AnalysisScope::File,
        };
    }
}
