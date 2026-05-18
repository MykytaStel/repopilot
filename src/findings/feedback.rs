use crate::findings::types::Finding;
use crate::scan::types::ScanSummary;
use serde::Serialize;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const FEEDBACK_PATH: &str = ".repopilot/feedback.yml";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalSuppression {
    pub rule_id: String,
    pub path: String,
    pub reason: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, PartialEq, Eq)]
pub struct LocalFeedbackReport {
    pub feedback_path: Option<PathBuf>,
    pub suppressions_loaded: usize,
    pub suppressed_findings_count: usize,
}

pub fn apply_local_feedback(
    summary: &mut ScanSummary,
    root: &Path,
) -> io::Result<LocalFeedbackReport> {
    let feedback_path = root.join(FEEDBACK_PATH);

    if !feedback_path.is_file() {
        return Ok(LocalFeedbackReport::default());
    }

    let content = fs::read_to_string(&feedback_path)?;
    let suppressions = parse_suppressions(&content);

    if suppressions.is_empty() {
        return Ok(LocalFeedbackReport {
            feedback_path: Some(feedback_path),
            suppressions_loaded: 0,
            suppressed_findings_count: 0,
        });
    }

    let original_count = summary.findings.len();
    summary
        .findings
        .retain(|finding| !is_suppressed(finding, &suppressions));

    let suppressed_findings_count = original_count.saturating_sub(summary.findings.len());
    summary.visible_findings_count = summary.findings.len();
    summary.health_score =
        ScanSummary::compute_health_score(&summary.findings, summary.non_empty_lines);

    Ok(LocalFeedbackReport {
        feedback_path: Some(feedback_path),
        suppressions_loaded: suppressions.len(),
        suppressed_findings_count,
    })
}

pub fn parse_suppressions(content: &str) -> Vec<LocalSuppression> {
    let mut suppressions = Vec::new();
    let mut current_rule_id: Option<String> = None;
    let mut current_path: Option<String> = None;
    let mut current_reason: Option<String> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed == "suppressions:" {
            continue;
        }

        if let Some(value) = trimmed.strip_prefix("- rule_id:") {
            push_current(
                &mut suppressions,
                &mut current_rule_id,
                &mut current_path,
                &mut current_reason,
            );
            current_rule_id = Some(clean_yaml_value(value));
            continue;
        }

        if let Some(value) = trimmed.strip_prefix("rule_id:") {
            current_rule_id = Some(clean_yaml_value(value));
            continue;
        }

        if let Some(value) = trimmed.strip_prefix("path:") {
            current_path = Some(clean_yaml_value(value));
            continue;
        }

        if let Some(value) = trimmed.strip_prefix("reason:") {
            current_reason = Some(clean_yaml_value(value));
        }
    }

    push_current(
        &mut suppressions,
        &mut current_rule_id,
        &mut current_path,
        &mut current_reason,
    );
    suppressions
}

fn push_current(
    suppressions: &mut Vec<LocalSuppression>,
    rule_id: &mut Option<String>,
    path: &mut Option<String>,
    reason: &mut Option<String>,
) {
    let Some(rule_id_value) = rule_id.take() else {
        *path = None;
        *reason = None;
        return;
    };
    let Some(path_value) = path.take() else {
        *reason = None;
        return;
    };

    suppressions.push(LocalSuppression {
        rule_id: rule_id_value,
        path: normalize_path_text(&path_value),
        reason: reason.take(),
    });
}

fn clean_yaml_value(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string()
}

fn is_suppressed(finding: &Finding, suppressions: &[LocalSuppression]) -> bool {
    suppressions.iter().any(|suppression| {
        finding.rule_id == suppression.rule_id
            && finding.evidence.first().is_some_and(|evidence| {
                normalize_path_text(&evidence.path.to_string_lossy()) == suppression.path
            })
    })
}

fn normalize_path_text(path: &str) -> String {
    path.replace('\\', "/").trim_start_matches("./").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_feedback_suppressions() {
        let suppressions = parse_suppressions(
            r#"
suppressions:
  - rule_id: architecture.large-file
    path: src/generated/schema.rs
    reason: generated schema boundary
"#,
        );

        assert_eq!(suppressions.len(), 1);
        assert_eq!(suppressions[0].rule_id, "architecture.large-file");
        assert_eq!(suppressions[0].path, "src/generated/schema.rs");
        assert_eq!(
            suppressions[0].reason.as_deref(),
            Some("generated schema boundary")
        );
    }
}
