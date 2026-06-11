use crate::analysis::{FileRole, classify_file_architecture, parse::ParsedFile};
use crate::audits::code_quality::function_spans::for_each_function;
use crate::audits::traits::FileAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::scan::config::ScanConfig;
use crate::scan::facts::FileFacts;
use crate::scan::path_classification::is_low_signal_audit_path;
use std::path::Path;
use tree_sitter::Node;

mod score;

#[cfg(test)]
mod tests;

const RULE_ID: &str = "code-quality.complex-function";

/// Flags individual functions whose control flow is deeply nested or
/// branch-heavy, using a cognitive-complexity-lite score that weights nesting.
/// Unlike `code-quality.complex-file`, a wide-but-flat dispatcher scores low.
pub struct ComplexFunctionAudit;

impl FileAudit for ComplexFunctionAudit {
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

impl ComplexFunctionAudit {
    fn analyze(&self, file: &FileFacts, parsed: &ParsedFile, config: &ScanConfig) -> Vec<Finding> {
        let arch_ctx = classify_file_architecture(file, config);
        if arch_ctx.file_role != FileRole::Production {
            return vec![];
        }

        if is_low_signal_audit_path(&file.path) {
            return vec![];
        }

        let Some(content) = file.content.as_deref() else {
            return vec![];
        };
        if content.trim().is_empty() {
            return vec![];
        }

        let Some(language) = file.language.as_deref() else {
            return vec![];
        };

        // Cognitive scoring needs the syntax tree; there is no heuristic fallback.
        let Some(tree) = parsed.tree() else {
            return vec![];
        };

        let threshold = config.complex_function_threshold;
        let mut findings = Vec::new();
        for_each_function(tree, content, language, &mut |node, name, _is_anonymous| {
            let complexity = score::cognitive_score(node, language);
            if complexity > threshold {
                findings.push(build_finding(&file.path, node, name, complexity, threshold));
            }
        });
        findings
    }
}

fn build_finding(
    path: &Path,
    node: Node<'_>,
    name: &str,
    score: usize,
    threshold: usize,
) -> Finding {
    let start_row = node.start_position().row + 1;
    let end_row = node.end_position().row + 1;
    let label = if name.is_empty() {
        "anonymous function".to_string()
    } else {
        format!("`{name}`")
    };
    let snippet_name = if name.is_empty() { "<anonymous>" } else { name };

    Finding {
        id: String::new(),
        rule_id: RULE_ID.to_string(),
        recommendation: Finding::recommendation_for_rule_id(RULE_ID),
        title: format!("Complex function: {label}"),
        description: format!(
            "This function's cognitive complexity score is {score}, exceeding the configured threshold of {threshold}. Deeply nested branches and loops are harder to follow than flat ones; consider extracting nested blocks into helpers or using early returns."
        ),
        category: FindingCategory::CodeQuality,
        // Severity and confidence are owned by the rule registry (single source
        // of truth); Medium matches the registry default.
        severity: Severity::Medium,
        confidence: Default::default(),
        evidence: vec![Evidence {
            path: path.to_path_buf(),
            line_start: start_row,
            line_end: Some(end_row),
            snippet: format!("fn {snippet_name}: score={score} (threshold={threshold})"),
        }],
        workspace_package: None,
        docs_url: None,
        provenance: Default::default(),
        risk: Default::default(),
    }
}
