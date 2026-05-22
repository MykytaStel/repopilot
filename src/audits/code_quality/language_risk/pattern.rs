use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::knowledge::decision::decide_for_file;
use crate::scan::facts::FileFacts;
use std::path::Path;

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
                let decision = decide_for_file(
                    $pattern.rule_id(),
                    file,
                    $pattern.base_severity(),
                    Some($pattern.signal()),
                );
                if !decision.is_suppressed() {
                    findings.push(build_finding(
                        path,
                        line_index + 1,
                        raw_line.trim(),
                        $pattern,
                        decision.severity,
                    ));
                }
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
