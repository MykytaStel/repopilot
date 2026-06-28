use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::knowledge::decision::{decide_for_file, record_decision_provenance};
use crate::scan::facts::FileFacts;
use std::path::Path;
use tree_sitter::{Node, Tree};

const RUNTIME_ERROR_HANDLING_DOCS_URL: &str =
    "https://owasp.org/www-community/vulnerabilities/Improper_Error_Handling";

#[path = "go.rs"]
mod go;
#[path = "js.rs"]
mod js;
#[path = "managed.rs"]
mod managed;
#[path = "python.rs"]
mod python;

use go::GoRiskPattern;
use js::JsRiskPattern;
use managed::ManagedRiskPattern;
use python::PythonRiskPattern;

pub(super) fn emit_findings_for_line(
    language_id: &str,
    trimmed: &str,
    path: &Path,
    raw_line: &str,
    line_index: usize,
    file: &FileFacts,
    findings: &mut Vec<Finding>,
) {
    macro_rules! push_if_matched {
        ($pattern:expr) => {
            if $pattern.matches(trimmed, path) {
                push_pattern_finding(
                    $pattern,
                    path,
                    line_index + 1,
                    raw_line.trim(),
                    file,
                    findings,
                );
            }
        };
    }

    match language_id {
        "go" => {
            for p in GoRiskPattern::ALL {
                push_if_matched!(p);
            }
        }
        "python" => {
            for p in PythonRiskPattern::ALL {
                push_if_matched!(p);
            }
        }
        "typescript" | "typescript-react" | "javascript" | "javascript-react" => {
            for p in JsRiskPattern::ALL {
                push_if_matched!(p);
            }
        }
        "java" | "kotlin" => {
            for p in ManagedRiskPattern::JVM_PATTERNS {
                push_if_matched!(p);
            }
        }
        "csharp" => {
            for p in ManagedRiskPattern::CSHARP_PATTERNS {
                push_if_matched!(p);
            }
        }
        _ => {}
    }
}

// ── AST detection (parseable languages) ────────────────────────────────────────

/// Emits runtime-risk findings for `language_id` by walking the parsed syntax
/// tree, so matches reflect real call/clause structure rather than line text.
/// Each language's node-level emitter lives in its sibling pattern module
/// (`go`, `js`, `python`, `managed`); this dispatches to them and recurses.
pub(super) fn emit_findings_for_tree(
    language_id: &str,
    tree: &Tree,
    content: &str,
    path: &Path,
    file: &FileFacts,
    findings: &mut Vec<Finding>,
) {
    walk_tree(tree.root_node(), language_id, content, path, file, findings);
}

fn walk_tree(
    node: Node<'_>,
    language_id: &str,
    content: &str,
    path: &Path,
    file: &FileFacts,
    findings: &mut Vec<Finding>,
) {
    match language_id {
        "python" => python::emit_python_node(node, content, path, file, findings),
        "go" => go::emit_go_node(node, content, path, file, findings),
        "typescript" | "typescript-react" | "javascript" | "javascript-react" => {
            js::emit_js_node(node, content, path, file, findings)
        }
        "java" => managed::emit_java_node(node, content, path, file, findings),
        "kotlin" => managed::emit_kotlin_node(node, content, path, file, findings),
        "csharp" => managed::emit_csharp_node(node, content, path, file, findings),
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_tree(child, language_id, content, path, file, findings);
    }
}

// ── Shared node helpers (used by the per-language emitters) ─────────────────────

fn line_of(node: Node<'_>) -> usize {
    node.start_position().row + 1
}

fn snippet_of(node: Node<'_>, content: &str) -> String {
    content
        .lines()
        .nth(node.start_position().row)
        .unwrap_or("")
        .trim()
        .to_string()
}

fn node_text<'a>(node: Node<'_>, content: &'a str) -> Option<&'a str> {
    node.utf8_text(content.as_bytes()).ok()
}

// ── Shared finding construction ─────────────────────────────────────────────────

trait RiskPattern {
    fn matches(&self, trimmed: &str, path: &Path) -> bool;
    fn rule_id(&self) -> &'static str;
    fn signal(&self) -> &'static str;
    fn title(&self) -> &'static str;
    fn context_label(&self) -> &'static str;
    fn recommendation(&self) -> &'static str;
    fn base_severity(&self) -> Severity;
}

macro_rules! impl_risk_pattern {
    ($t:ty) => {
        impl RiskPattern for $t {
            fn matches(&self, trimmed: &str, path: &Path) -> bool {
                (*self).matches(trimmed, path)
            }
            fn rule_id(&self) -> &'static str {
                (*self).rule_id()
            }
            fn signal(&self) -> &'static str {
                (*self).signal()
            }
            fn title(&self) -> &'static str {
                (*self).title()
            }
            fn context_label(&self) -> &'static str {
                (*self).context_label()
            }
            fn recommendation(&self) -> &'static str {
                (*self).recommendation()
            }
            fn base_severity(&self) -> Severity {
                (*self).base_severity()
            }
        }
    };
}

impl_risk_pattern!(GoRiskPattern);
impl_risk_pattern!(PythonRiskPattern);
impl_risk_pattern!(JsRiskPattern);
impl_risk_pattern!(ManagedRiskPattern);

/// Applies the per-file decision and pushes a finding for a matched pattern.
/// Shared by the line scanner and the AST walker so both produce identical
/// findings (title, recommendation, severity decision) for the same pattern.
fn push_pattern_finding(
    pattern: &dyn RiskPattern,
    path: &Path,
    line_number: usize,
    snippet: &str,
    file: &FileFacts,
    findings: &mut Vec<Finding>,
) {
    let base_severity = pattern.base_severity();
    let signal = pattern.signal();
    let decision = decide_for_file(pattern.rule_id(), file, base_severity, Some(signal));
    if !decision.is_suppressed() {
        let mut finding = build_finding(path, line_number, snippet, pattern, decision.severity);
        record_decision_provenance(&mut finding, base_severity, Some(signal), &decision);
        findings.push(finding);
    }
}

fn build_finding(
    path: &Path,
    line_number: usize,
    snippet: &str,
    pattern: &dyn RiskPattern,
    severity: Severity,
) -> Finding {
    Finding {
        id: String::new(),
        rule_id: pattern.rule_id().to_string(),
        recommendation: pattern.recommendation().to_string(),
        title: pattern.title().to_string(),
        description: format!(
            "{} was found in {}. Runtime termination and placeholder exception paths can bypass normal error handling and make failures harder to recover from.",
            pattern.context_label(),
            path.display(),
        ),
        category: FindingCategory::CodeQuality,
        severity,
        confidence: Default::default(),
        evidence: vec![Evidence {
            path: path.to_path_buf(),
            line_start: line_number,
            line_end: None,
            snippet: snippet.to_string(),
        }],
        workspace_package: None,
        docs_url: Some(RUNTIME_ERROR_HANDLING_DOCS_URL.to_string()),
        provenance: Default::default(),
        risk: Default::default(),
    }
}
