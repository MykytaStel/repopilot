use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::knowledge::decision::decide_for_file;
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
/// Only the AST-backed languages (Python, TypeScript/JavaScript) are handled
/// here; other languages keep the line scanner in [`emit_findings_for_line`].
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
        "python" => emit_python_node(node, content, path, file, findings),
        "go" => emit_go_node(node, content, path, file, findings),
        "typescript" | "typescript-react" | "javascript" | "javascript-react" => {
            emit_js_node(node, content, path, file, findings)
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_tree(child, language_id, content, path, file, findings);
    }
}

fn emit_go_node(
    node: Node<'_>,
    content: &str,
    path: &Path,
    file: &FileFacts,
    findings: &mut Vec<Finding>,
) {
    if node.kind() != "call_expression" {
        return;
    }
    let Some(function) = node.child_by_field_name("function") else {
        return;
    };

    let pattern = match function.kind() {
        // `panic(...)`
        "identifier" if node_text(function, content) == Some("panic") => Some(GoRiskPattern::Panic),
        // `pkg.Fn(...)` — `log.Fatal`/`log.Fatalf` and `os.Exit`.
        "selector_expression" => {
            let package = function
                .child_by_field_name("operand")
                .and_then(|n| node_text(n, content));
            let method = function
                .child_by_field_name("field")
                .and_then(|n| node_text(n, content));
            match (package, method) {
                (Some("log"), Some("Fatal" | "Fatalf")) => Some(GoRiskPattern::LogFatal),
                (Some("os"), Some("Exit")) => Some(GoRiskPattern::OsExit),
                _ => None,
            }
        }
        _ => None,
    };

    if let Some(pattern) = pattern {
        push_pattern_finding(
            &pattern,
            path,
            line_of(node),
            &snippet_of(node, content),
            file,
            findings,
        );
    }
}

fn emit_js_node(
    node: Node<'_>,
    content: &str,
    path: &Path,
    file: &FileFacts,
    findings: &mut Vec<Finding>,
) {
    let pattern = match node.kind() {
        "call_expression" if is_process_exit_call(node, content) => {
            Some(JsRiskPattern::ProcessExit)
        }
        "throw_statement" if throws_new_error(node, content) && is_library_boundary_path(path) => {
            Some(JsRiskPattern::ThrowError)
        }
        _ => None,
    };
    if let Some(pattern) = pattern {
        push_pattern_finding(
            &pattern,
            path,
            line_of(node),
            &snippet_of(node, content),
            file,
            findings,
        );
    }
}

fn emit_python_node(
    node: Node<'_>,
    content: &str,
    path: &Path,
    file: &FileFacts,
    findings: &mut Vec<Finding>,
) {
    let pattern = match node.kind() {
        "except_clause" if is_bare_except(node) => Some(PythonRiskPattern::BroadExcept),
        "assert_statement" => Some(PythonRiskPattern::Assert),
        "call" if is_not_implemented_call(node, content) => Some(PythonRiskPattern::NotImplemented),
        "raise_statement" if raises_bare_not_implemented(node, content) => {
            Some(PythonRiskPattern::NotImplemented)
        }
        _ => None,
    };
    if let Some(pattern) = pattern {
        push_pattern_finding(
            &pattern,
            path,
            line_of(node),
            &snippet_of(node, content),
            file,
            findings,
        );
    }
}

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

/// `process.exit(...)` — a call whose callee is the `process.exit` member.
fn is_process_exit_call(node: Node<'_>, content: &str) -> bool {
    let Some(function) = node.child_by_field_name("function") else {
        return false;
    };
    if function.kind() != "member_expression" {
        return false;
    }
    let object = function
        .child_by_field_name("object")
        .and_then(|n| node_text(n, content));
    let property = function
        .child_by_field_name("property")
        .and_then(|n| node_text(n, content));
    object == Some("process") && property == Some("exit")
}

/// `throw new Error(...)` — a throw whose direct expression constructs `Error`.
fn throws_new_error(node: Node<'_>, content: &str) -> bool {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .any(|child| is_new_error(child, content))
}

fn is_new_error(node: Node<'_>, content: &str) -> bool {
    node.kind() == "new_expression"
        && node
            .child_by_field_name("constructor")
            .and_then(|c| node_text(c, content))
            == Some("Error")
}

/// A bare `except:` clause — an `except_clause` with no exception type before
/// its body block.
fn is_bare_except(node: Node<'_>) -> bool {
    let mut cursor = node.walk();
    !node
        .named_children(&mut cursor)
        .any(|child| child.kind() != "block" && child.kind() != "comment")
}

/// A call to `NotImplementedError(...)`.
fn is_not_implemented_call(node: Node<'_>, content: &str) -> bool {
    node.child_by_field_name("function")
        .and_then(|function| node_text(function, content))
        .map(|text| text == "NotImplementedError" || text.ends_with(".NotImplementedError"))
        .unwrap_or(false)
}

/// `raise NotImplementedError` with no call — the raised expression is the bare
/// `NotImplementedError` identifier (the call form is handled separately so a
/// single `raise NotImplementedError(...)` is not counted twice).
fn raises_bare_not_implemented(node: Node<'_>, content: &str) -> bool {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).any(|child| {
        child.kind() == "identifier" && node_text(child, content) == Some("NotImplementedError")
    })
}

// ── Shared ────────────────────────────────────────────────────────────────────

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
    let decision = decide_for_file(
        pattern.rule_id(),
        file,
        pattern.base_severity(),
        Some(pattern.signal()),
    );
    if !decision.is_suppressed() {
        findings.push(build_finding(
            path,
            line_number,
            snippet,
            pattern,
            decision.severity,
        ));
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

fn is_library_boundary_path(path: &Path) -> bool {
    path.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .map(|value| {
                matches!(
                    value.to_lowercase().as_str(),
                    "domain"
                        | "domains"
                        | "core"
                        | "model"
                        | "models"
                        | "lib"
                        | "libs"
                        | "packages"
                )
            })
            .unwrap_or(false)
    })
}
