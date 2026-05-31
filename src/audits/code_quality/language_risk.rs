use crate::analysis::parse::ParsedFile;
use crate::audits::code_quality::sanitize::{sanitize_c_style, sanitize_python_line};
use crate::audits::traits::FileAudit;
use crate::findings::provenance::{AnalysisScope, FindingProvenance};
use crate::findings::types::Finding;
use crate::knowledge::language::language_id_for_name;
use crate::rules::{RuleLifecycle, SignalSource, lookup_rule_metadata};
use crate::scan::config::ScanConfig;
use crate::scan::facts::FileFacts;
use std::path::Path;

pub struct LanguageRiskAudit;

impl FileAudit for LanguageRiskAudit {
    fn audit(&self, file: &FileFacts, config: &ScanConfig) -> Vec<Finding> {
        // Direct entry (tests, non-pipeline callers): build a one-off parse view.
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

impl LanguageRiskAudit {
    fn analyze(&self, file: &FileFacts, parsed: &ParsedFile, _config: &ScanConfig) -> Vec<Finding> {
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

        if !is_ast_runtime_language(language_id) {
            // Languages without an AST runtime detector (Go, Java/Kotlin, C#)
            // keep the sanitized line scanner and text-heuristic provenance.
            return detect_language_risks(language_id, content, &file.path, file);
        }

        match parsed.tree() {
            Some(tree) => {
                let mut findings = Vec::new();
                emit_findings_for_tree(language_id, tree, content, &file.path, file, &mut findings);
                findings
            }
            None => {
                // The file did not parse; fall back to the line scanner but keep
                // provenance honest by stamping the findings as text-heuristic.
                let mut findings = detect_language_risks(language_id, content, &file.path, file);
                mark_text_heuristic(&mut findings);
                findings
            }
        }
    }
}

/// Runtime-risk languages with an AST detector (matched from the syntax tree).
fn is_ast_runtime_language(language_id: &str) -> bool {
    matches!(
        language_id,
        "python"
            | "go"
            | "typescript"
            | "typescript-react"
            | "javascript"
            | "javascript-react"
            | "java"
            | "kotlin"
            | "csharp"
    )
}

/// Overrides provenance for line-scanner fallback findings so they report
/// `text-heuristic` rather than inheriting the rule's `ast` signal source.
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

use self::pattern::{emit_findings_for_line, emit_findings_for_tree};

#[cfg(test)]
mod tests;
