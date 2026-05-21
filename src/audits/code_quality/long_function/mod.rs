use crate::audits::context::{AuditContext, FileRole, FrameworkKind, LanguageKind, classify_file};
use crate::audits::traits::FileAudit;
use crate::findings::types::{Confidence, Evidence, Finding, FindingCategory, Severity};
use crate::knowledge::decision::decide_for_audit_context;
use crate::scan::config::ScanConfig;
use crate::scan::facts::FileFacts;
use crate::scan::path_classification::is_low_signal_audit_path;
use std::path::Path;

mod brace;
mod python;

const RULE_ID: &str = "code-quality.long-function";

pub struct LongFunctionAudit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LongFunctionPolicy {
    pub threshold: usize,
    pub severity: Severity,
    pub confidence: Confidence,
    pub context_label: &'static str,
    pub confidence_reason: &'static str,
    pub recommendation: &'static str,
}

impl FileAudit for LongFunctionAudit {
    fn audit(&self, file: &FileFacts, config: &ScanConfig) -> Vec<Finding> {
        if is_low_signal_audit_path(&file.path) {
            return vec![];
        }

        let content = file.content.as_deref().unwrap_or("");

        if content.is_empty() {
            return vec![];
        }

        let Some(language) = file.language.as_deref() else {
            return vec![];
        };

        let context = classify_file(file);
        let decision = decide_for_audit_context(RULE_ID, &context, Severity::Medium, None);

        if decision.is_suppressed() {
            return vec![];
        }

        let mut policy = long_function_policy(&context, config.long_function_loc_threshold);
        policy.severity = decision.severity.min(policy.severity);

        detect_long_functions(content, language, &file.path, policy)
    }
}

fn detect_long_functions(
    content: &str,
    language: &str,
    path: &Path,
    policy: LongFunctionPolicy,
) -> Vec<Finding> {
    if language == "Python" {
        python::detect_python(content, path, policy)
    } else {
        brace::detect_brace_based(content, language, path, policy)
    }
}

fn long_function_policy(context: &AuditContext, base_threshold: usize) -> LongFunctionPolicy {
    if context.has_role(FileRole::Config) {
        return LongFunctionPolicy {
            threshold: usize::MAX,
            severity: Severity::Info,
            confidence: Confidence::Low,
            context_label: "configuration file",
            confidence_reason: "configuration files often encode declarative setup rather than executable business logic",
            recommendation: "Configuration files are not evaluated with the generic long-function threshold.",
        };
    }

    if context.is_react_component() {
        return LongFunctionPolicy {
            threshold: base_threshold.saturating_mul(3),
            severity: Severity::Low,
            confidence: Confidence::Low,
            context_label: "React component",
            confidence_reason: "JSX, hooks, and layout markup often make component files longer without implying mixed responsibilities",
            recommendation: "Split only if state, effects, rendering, or data-shaping concerns are mixed. Prefer extracting child components, hooks, or view-model helpers at clear boundaries.",
        };
    }

    if context.is_react_hook() {
        return LongFunctionPolicy {
            threshold: base_threshold.saturating_mul(2),
            severity: Severity::Low,
            confidence: Confidence::Low,
            context_label: "React hook",
            confidence_reason: "hooks often combine state and effect orchestration that can be longer than a pure helper function",
            recommendation: "For large hooks, consider splitting state/effect orchestration from data mapping or side-effect helpers.",
        };
    }

    if context.has_role(FileRole::UnityMonoBehaviour) || context.has_framework(FrameworkKind::Unity)
    {
        return LongFunctionPolicy {
            threshold: base_threshold.saturating_mul(2),
            severity: Severity::Low,
            confidence: Confidence::Low,
            context_label: "Unity MonoBehaviour",
            confidence_reason: "engine lifecycle methods can accumulate setup and event wiring that is not always business logic",
            recommendation: "For large Unity behaviours, consider moving domain logic out of lifecycle methods and into smaller services or components.",
        };
    }

    if context.has_role(FileRole::DotNetController) {
        return LongFunctionPolicy {
            threshold: base_threshold.saturating_mul(2),
            severity: Severity::Low,
            confidence: Confidence::Low,
            context_label: ".NET controller",
            confidence_reason: "controller actions often include request/response wiring that can be longer than domain logic",
            recommendation: "For large controllers, prefer moving business logic into services, handlers, or application-layer use cases.",
        };
    }

    if context.has_role(FileRole::DotNetService) {
        return LongFunctionPolicy {
            threshold: base_threshold.saturating_mul(3) / 2,
            severity: Severity::Medium,
            confidence: Confidence::High,
            context_label: ".NET service",
            confidence_reason: "service classes usually carry reusable production orchestration where long methods signal mixed responsibilities",
            recommendation: "For large services, consider splitting orchestration, validation, persistence, and external integration logic.",
        };
    }

    if context.has_role(FileRole::RustTest) || context.is_test {
        return LongFunctionPolicy {
            threshold: base_threshold.saturating_mul(2),
            severity: Severity::Low,
            confidence: Confidence::Low,
            context_label: "test code",
            confidence_reason: "tests often include setup and assertions that are naturally longer than production helpers",
            recommendation: "For large tests, consider extracting builders, fixtures, or assertion helpers, but test setup can be naturally longer than production logic.",
        };
    }

    if context.language == LanguageKind::Rust {
        return LongFunctionPolicy {
            threshold: base_threshold,
            severity: Severity::Medium,
            confidence: Confidence::High,
            context_label: "Rust production code",
            confidence_reason: "production Rust functions are usually explicit control-flow units where length increases review and error-handling risk",
            recommendation: "Consider extracting smaller functions or methods to isolate parsing, validation, IO, or transformation steps.",
        };
    }

    LongFunctionPolicy {
        threshold: base_threshold,
        severity: Severity::Medium,
        confidence: Confidence::High,
        context_label: "generic production code",
        confidence_reason: "production functions that exceed the threshold are more likely to mix responsibilities",
        recommendation: "Long functions are harder to test and reason about; consider extracting helper functions.",
    }
}

fn build_finding(
    path: &Path,
    start_line: usize,
    end_line: usize,
    fn_name: &str,
    fn_len: usize,
    policy: LongFunctionPolicy,
) -> Finding {
    let (title, name_display) = if fn_name.is_empty() {
        (
            format!("Large {} inline function", policy.context_label),
            "inline function".to_string(),
        )
    } else {
        (
            format!("Large {} function: `{fn_name}`", policy.context_label),
            format!("`{fn_name}`"),
        )
    };

    Finding {
        id: String::new(),
        rule_id: RULE_ID.to_string(),
        recommendation: policy.recommendation.to_string(),
        title,
        description: format!(
            "This is a long function in {context_label}; confidence is {confidence} because {confidence_reason}. Function {name_display} spans {fn_len} lines, exceeding the context-aware {threshold}-line threshold. Large functions are harder to review, test, and safely change.",
            confidence = policy.confidence.label(),
            confidence_reason = policy.confidence_reason,
            threshold = policy.threshold,
            context_label = policy.context_label,
        ),
        category: FindingCategory::CodeQuality,
        severity: policy.severity,
        confidence: policy.confidence,
        evidence: vec![Evidence {
            path: path.to_path_buf(),
            line_start: start_line,
            line_end: Some(end_line),
            snippet: format!(
                "function spans lines {start_line}–{end_line} ({fn_len} lines, threshold {})",
                policy.threshold
            ),
        }],
        workspace_package: None,
        docs_url: None,
        provenance: Default::default(),
        risk: Default::default(),
    }
}

#[cfg(test)]
mod tests;
