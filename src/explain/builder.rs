use crate::audits::context::classify_file;
use crate::explain::model::{
    ExplainContext, ExplainDecision, ExplainReport, ExplainRiskSignal, ExplainSource,
};
use crate::findings::types::Severity;
use crate::knowledge::decision::decide_for_file;
use crate::knowledge::language::{detect_language_for_path, profile_by_id};
use crate::knowledge::model::{RuleDecisionAction, SupportLevel};
use crate::scan::facts::FileFacts;
use std::fs;
use std::io;
use std::path::Path;

pub fn build_explain_report(
    path: &Path,
    rule_id: Option<&str>,
    signal: Option<&str>,
    base_severity: Severity,
) -> Result<ExplainReport, io::Error> {
    let content = fs::read_to_string(path)?;
    let language_name = detect_language_for_path(path).map(str::to_string);
    let non_empty_lines = count_non_empty_lines(&content);
    let has_inline_tests = has_language_inline_tests(path, language_name.as_deref(), &content);

    let file = FileFacts {
        path: path.to_path_buf(),
        language: language_name.clone(),
        non_empty_lines,
        branch_count: 0,
        imports: Vec::new(),
        content: Some(content),
        has_inline_tests,
    };

    let audit_context = classify_file(&file);
    let language_id = audit_context.language_id();
    let language_support =
        profile_by_id(language_id).map(|profile| support_level_label(profile.support).to_string());

    let context = ExplainContext {
        language: language_id.to_string(),
        language_support,
        frameworks: audit_context
            .framework_ids()
            .into_iter()
            .map(str::to_string)
            .collect(),
        roles: audit_context
            .role_ids()
            .into_iter()
            .map(str::to_string)
            .collect(),
        paradigms: audit_context
            .paradigm_ids()
            .into_iter()
            .map(str::to_string)
            .collect(),
        runtimes: audit_context
            .runtime_ids()
            .into_iter()
            .map(str::to_string)
            .collect(),
        is_test: audit_context.is_test,
        is_production_code: audit_context.is_production_code(),
    };

    let decision = rule_id.map(|rule_id| {
        let decision = decide_for_file(rule_id, &file, base_severity, signal);

        ExplainDecision {
            rule_id: rule_id.to_string(),
            signal: signal.map(str::to_string),
            base_severity,
            action: decision_action_label(decision.action).to_string(),
            final_severity: decision.severity,
            reason: decision.reason,
            risk_signal: decision.risk_signal.map(|signal| ExplainRiskSignal {
                id: signal.id,
                label: signal.label,
                weight: signal.weight,
                reason: signal.reason,
            }),
        }
    });

    Ok(ExplainReport {
        path: path.display().to_string(),
        source: ExplainSource {
            language_name,
            non_empty_lines,
            has_inline_tests,
        },
        context,
        decision,
    })
}

fn count_non_empty_lines(content: &str) -> usize {
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count()
}

fn has_language_inline_tests(path: &Path, language: Option<&str>, content: &str) -> bool {
    match language {
        Some("Rust") => content.contains("#[cfg(test)]") || content.contains("#[test]"),
        Some("TypeScript")
        | Some("TypeScript React")
        | Some("JavaScript")
        | Some("JavaScript React") => {
            contains_call(content, "describe")
                || contains_call(content, "it")
                || contains_call(content, "test")
        }
        Some("Python") => content.contains("def test_") || content.contains("unittest."),
        Some("Go") => content.contains("func Test") || content.contains("func Benchmark"),
        Some("Java") | Some("Kotlin") => content.contains("@Test"),
        _ => path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.contains("_test.") || name.contains(".test.")),
    }
}

fn contains_call(content: &str, name: &str) -> bool {
    let needle = format!("{name}(");
    content.match_indices(&needle).any(|(index, _)| {
        content[..index]
            .chars()
            .next_back()
            .is_none_or(|ch| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '.')
    })
}

fn support_level_label(level: SupportLevel) -> &'static str {
    match level {
        SupportLevel::DetectOnly => "detect-only",
        SupportLevel::ImportAware => "import-aware",
        SupportLevel::ContextAware => "context-aware",
        SupportLevel::RuleAware => "rule-aware",
    }
}

fn decision_action_label(action: RuleDecisionAction) -> &'static str {
    match action {
        RuleDecisionAction::Apply => "apply",
        RuleDecisionAction::Suppress => "suppress",
        RuleDecisionAction::Downgrade => "downgrade",
        RuleDecisionAction::Upgrade => "upgrade",
    }
}
