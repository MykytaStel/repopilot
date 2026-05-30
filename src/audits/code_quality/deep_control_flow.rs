use crate::analysis::{FileRole, classify_file_architecture, parse::ParsedFile};
use crate::audits::traits::FileAudit;
use crate::findings::types::{Confidence, Evidence, Finding, FindingCategory, Severity};
use crate::knowledge::decision::apply_file_decision;
use crate::scan::config::ScanConfig;
use crate::scan::facts::FileFacts;
use std::path::Path;
use tree_sitter::Node;

pub struct DeepControlFlowAudit;

impl FileAudit for DeepControlFlowAudit {
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

impl DeepControlFlowAudit {
    fn analyze(&self, file: &FileFacts, parsed: &ParsedFile, config: &ScanConfig) -> Vec<Finding> {
        let arch_ctx = classify_file_architecture(file, config);
        if arch_ctx.file_role != FileRole::Production {
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

        let Some(tree) = parsed.tree() else {
            return vec![];
        };

        let mut findings = Vec::new();
        visit(
            tree.root_node(),
            0,
            config.max_control_flow_depth,
            language,
            content,
            &file.path,
            &mut findings,
        );

        findings
            .into_iter()
            .filter_map(|finding| {
                apply_file_decision("code-quality.deep-control-flow", file, finding, None)
            })
            .collect()
    }
}

fn is_control_flow_node(kind: &str, language: &str) -> bool {
    match language {
        "Rust" => matches!(
            kind,
            "if_expression"
                | "for_expression"
                | "while_expression"
                | "loop_expression"
                | "match_expression"
        ),
        "Python" => matches!(
            kind,
            "if_statement"
                | "for_statement"
                | "while_statement"
                | "match_statement"
                | "try_statement"
        ),
        _ => matches!(
            kind,
            "if_statement"
                | "for_statement"
                | "for_in_statement"
                | "for_of_statement"
                | "while_statement"
                | "do_statement"
                | "switch_statement"
                | "try_statement"
        ),
    }
}

fn is_else_if(node: Node<'_>, language: &str) -> bool {
    let kind = node.kind();
    if (language == "Rust" && kind == "if_expression")
        || (language != "Rust" && kind == "if_statement")
    {
        if let Some(parent) = node.parent() {
            if parent.kind() == "else_clause" {
                return true;
            }
        }
    }
    false
}

fn visit(
    node: Node<'_>,
    current_depth: usize,
    max_depth: usize,
    language: &str,
    content: &str,
    path: &Path,
    findings: &mut Vec<Finding>,
) {
    let is_cf = is_control_flow_node(node.kind(), language);
    let mut next_depth = current_depth;
    if is_cf && !is_else_if(node, language) {
        next_depth += 1;
        if next_depth > max_depth && current_depth <= max_depth {
            findings.push(build_finding(node, next_depth, max_depth, content, path));
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit(
            child, next_depth, max_depth, language, content, path, findings,
        );
    }
}

fn build_finding(
    node: Node<'_>,
    depth: usize,
    threshold: usize,
    content: &str,
    path: &Path,
) -> Finding {
    let start_pos = node.start_position();
    let line_start = start_pos.row + 1;
    let snippet = content
        .lines()
        .nth(start_pos.row)
        .unwrap_or("")
        .trim()
        .to_string();

    Finding {
        id: String::new(),
        rule_id: "code-quality.deep-control-flow".to_string(),
        recommendation: Finding::recommendation_for_rule_id("code-quality.deep-control-flow"),
        title: "Deep control flow nesting detected".to_string(),
        description: format!(
            "Control flow blocks are nested {depth} levels deep, which exceeds the configured limit of {threshold}. Consider refactoring by extracting nested blocks or using early returns to simplify control flow."
        ),
        category: FindingCategory::CodeQuality,
        severity: Severity::Low,
        confidence: Confidence::Medium,
        evidence: vec![Evidence {
            path: path.to_path_buf(),
            line_start,
            line_end: None,
            snippet,
        }],
        workspace_package: None,
        docs_url: None,
        provenance: Default::default(),
        risk: Default::default(),
    }
}
