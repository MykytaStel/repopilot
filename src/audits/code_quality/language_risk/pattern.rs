use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::knowledge::decision::{decide_for_file, record_decision_provenance};
use crate::scan::facts::FileFacts;
use std::path::Path;
use tree_sitter::Node;

const RUNTIME_ERROR_HANDLING_DOCS_URL: &str =
    "https://owasp.org/www-community/vulnerabilities/Improper_Error_Handling";

#[path = "go.rs"]
pub(crate) mod go;
#[path = "js.rs"]
pub(crate) mod js;
#[path = "managed.rs"]
pub(crate) mod managed;
#[path = "python.rs"]
pub(crate) mod python;

use go::GoRiskPattern;
use js::JsRiskPattern;
use managed::ManagedRiskPattern;
use python::PythonRiskPattern;

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
