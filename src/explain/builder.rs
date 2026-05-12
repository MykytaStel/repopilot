use crate::audits::context::classify_file;
use crate::explain::model::{ExplainContext, ExplainDecision, ExplainReport, ExplainSource};
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
    let lines_of_code = count_non_empty_lines(&content);
    let has_inline_tests = content.contains("#[cfg(test)]");

    let file = FileFacts {
        path: path.to_path_buf(),
        language: language_name.clone(),
        lines_of_code,
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
        }
    });

    Ok(ExplainReport {
        path: path.display().to_string(),
        source: ExplainSource {
            language_name,
            lines_of_code,
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
