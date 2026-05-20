use crate::findings::types::Finding;
use crate::scan::types::{ScanDiagnostic, ScanSummary};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const FEEDBACK_PATH: &str = ".repopilot/feedback.yml";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalSuppression {
    pub index: usize,
    pub rule_id: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalFeedbackReport {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feedback_path: Option<PathBuf>,
    pub suppressions_loaded: usize,
    pub suppressed_findings_count: usize,
    pub unmatched_suppressions_count: usize,
    pub invalid_suppressions_count: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unmatched_suppressions: Vec<LocalSuppression>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parse_error: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalFeedbackValidation {
    pub feedback_path: PathBuf,
    pub exists: bool,
    pub suppressions: Vec<LocalSuppression>,
    pub invalid_suppressions_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parse_error: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<ScanDiagnostic>,
}

#[derive(Debug, Default, Deserialize)]
struct RawFeedbackFile {
    #[serde(default)]
    suppressions: Vec<RawSuppression>,
}

#[derive(Debug, Default, Deserialize)]
struct RawSuppression {
    rule_id: Option<String>,
    path: Option<String>,
    reason: Option<String>,
}

pub fn apply_local_feedback(
    summary: &mut ScanSummary,
    root: &Path,
) -> io::Result<LocalFeedbackReport> {
    let validation = validate_local_feedback(root)?;

    if !validation.exists {
        return Ok(LocalFeedbackReport::default());
    }

    summary.diagnostics.extend(validation.diagnostics.clone());

    let mut report = LocalFeedbackReport {
        feedback_path: Some(validation.feedback_path.clone()),
        suppressions_loaded: validation.suppressions.len(),
        invalid_suppressions_count: validation.invalid_suppressions_count,
        parse_error: validation.parse_error.clone(),
        ..LocalFeedbackReport::default()
    };

    if validation.parse_error.is_some() || validation.suppressions.is_empty() {
        summary.local_feedback = Some(report.clone());
        return Ok(report);
    }

    let original_count = summary.findings.len();
    let mut matched_suppression_indices = BTreeSet::new();
    let suppression_index = build_suppression_index(&validation.suppressions);

    summary.findings.retain(|finding| {
        let matched = matching_suppression_index(finding, &suppression_index);
        if let Some(index) = matched {
            matched_suppression_indices.insert(index);
            false
        } else {
            true
        }
    });

    report.suppressed_findings_count = original_count.saturating_sub(summary.findings.len());
    report.unmatched_suppressions = validation
        .suppressions
        .iter()
        .filter(|suppression| !matched_suppression_indices.contains(&suppression.index))
        .cloned()
        .collect();
    report.unmatched_suppressions_count = report.unmatched_suppressions.len();

    if report.unmatched_suppressions_count > 0 {
        summary.diagnostics.push(
            ScanDiagnostic::warning(
                "feedback.unmatched-suppressions",
                format!(
                    "{} local feedback suppression(s) did not match current findings.",
                    report.unmatched_suppressions_count
                ),
            )
            .with_path(validation.feedback_path),
        );
    }

    summary.visible_findings_count = summary.findings.len();
    summary.health_score =
        ScanSummary::compute_health_score(&summary.findings, summary.non_empty_lines);
    summary.local_feedback = Some(report.clone());

    Ok(report)
}

pub fn validate_local_feedback(root: &Path) -> io::Result<LocalFeedbackValidation> {
    let feedback_path = root.join(FEEDBACK_PATH);

    if !feedback_path.is_file() {
        return Ok(LocalFeedbackValidation {
            feedback_path,
            exists: false,
            ..LocalFeedbackValidation::default()
        });
    }

    let content = fs::read_to_string(&feedback_path)?;
    Ok(validate_feedback_content(content.as_str(), feedback_path))
}

pub fn validate_feedback_content(content: &str, feedback_path: PathBuf) -> LocalFeedbackValidation {
    let parsed = match serde_norway::from_str::<RawFeedbackFile>(content) {
        Ok(parsed) => parsed,
        Err(error) => {
            let message = error.to_string();
            return LocalFeedbackValidation {
                feedback_path: feedback_path.clone(),
                exists: true,
                parse_error: Some(message.clone()),
                diagnostics: vec![
                    ScanDiagnostic::warning(
                        "feedback.parse-failed",
                        format!("Could not parse local feedback YAML: {message}"),
                    )
                    .with_path(feedback_path),
                ],
                ..LocalFeedbackValidation::default()
            };
        }
    };

    let mut suppressions = Vec::new();
    let mut diagnostics = Vec::new();
    let mut invalid_suppressions_count = 0;

    for (offset, raw) in parsed.suppressions.into_iter().enumerate() {
        let index = offset + 1;
        let rule_id = clean_optional_value(raw.rule_id);
        let path = clean_optional_value(raw.path);
        let reason = clean_optional_value(raw.reason);

        match (rule_id, path) {
            (Some(rule_id), Some(path)) => suppressions.push(LocalSuppression {
                index,
                rule_id,
                path: normalize_path_text(&path),
                reason,
            }),
            (rule_id, path) => {
                invalid_suppressions_count += 1;
                let missing = match (rule_id.is_none(), path.is_none()) {
                    (true, true) => "rule_id and path",
                    (true, false) => "rule_id",
                    (false, true) => "path",
                    (false, false) => "required field",
                };
                diagnostics.push(
                    ScanDiagnostic::warning(
                        "feedback.invalid-suppression",
                        format!("Suppression #{index} is missing {missing}."),
                    )
                    .with_path(feedback_path.clone()),
                );
            }
        }
    }

    LocalFeedbackValidation {
        feedback_path,
        exists: true,
        suppressions,
        invalid_suppressions_count,
        parse_error: None,
        diagnostics,
    }
}

pub fn parse_suppressions(content: &str) -> Vec<LocalSuppression> {
    validate_feedback_content(content, PathBuf::from(FEEDBACK_PATH)).suppressions
}

fn clean_optional_value(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct SuppressionKey {
    rule_id: String,
    path: String,
}

fn build_suppression_index(suppressions: &[LocalSuppression]) -> HashMap<SuppressionKey, usize> {
    let mut index = HashMap::new();
    for suppression in suppressions {
        index
            .entry(SuppressionKey {
                rule_id: suppression.rule_id.clone(),
                path: suppression.path.clone(),
            })
            .or_insert(suppression.index);
    }
    index
}

fn matching_suppression_index(
    finding: &Finding,
    suppressions: &HashMap<SuppressionKey, usize>,
) -> Option<usize> {
    finding
        .evidence
        .iter()
        .filter_map(|evidence| {
            let key = SuppressionKey {
                rule_id: finding.rule_id.clone(),
                path: normalize_path_text(&evidence.path.to_string_lossy()),
            };
            suppressions.get(&key).copied()
        })
        .min()
}

fn normalize_path_text(path: &str) -> String {
    path.replace('\\', "/").trim_start_matches("./").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::findings::types::{Evidence, Finding};

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
        assert_eq!(suppressions[0].index, 1);
        assert_eq!(suppressions[0].rule_id, "architecture.large-file");
        assert_eq!(suppressions[0].path, "src/generated/schema.rs");
        assert_eq!(
            suppressions[0].reason.as_deref(),
            Some("generated schema boundary")
        );
    }

    #[test]
    fn reports_malformed_yaml_as_warning() {
        let validation = validate_feedback_content(
            "suppressions:\n  - rule_id: [\n",
            PathBuf::from(".repopilot/feedback.yml"),
        );

        assert!(validation.parse_error.is_some());
        assert_eq!(validation.diagnostics.len(), 1);
        assert_eq!(validation.diagnostics[0].code, "feedback.parse-failed");
    }

    #[test]
    fn rejects_incomplete_suppressions() {
        let validation = validate_feedback_content(
            r#"
suppressions:
  - rule_id: security.secret-candidate
  - path: src/main.rs
"#,
            PathBuf::from(".repopilot/feedback.yml"),
        );

        assert_eq!(validation.suppressions.len(), 0);
        assert_eq!(validation.invalid_suppressions_count, 2);
        assert_eq!(validation.diagnostics.len(), 2);
    }

    #[test]
    fn indexed_matching_considers_all_evidence_paths_and_suppression_order() {
        let suppressions = vec![
            LocalSuppression {
                index: 1,
                rule_id: "security.secret-candidate".to_string(),
                path: "src/second.rs".to_string(),
                reason: None,
            },
            LocalSuppression {
                index: 2,
                rule_id: "security.secret-candidate".to_string(),
                path: "src/first.rs".to_string(),
                reason: None,
            },
            LocalSuppression {
                index: 3,
                rule_id: "architecture.large-file".to_string(),
                path: "src/first.rs".to_string(),
                reason: None,
            },
        ];
        let index = build_suppression_index(&suppressions);
        let finding = Finding {
            rule_id: "security.secret-candidate".to_string(),
            evidence: vec![
                Evidence {
                    path: PathBuf::from("./src/first.rs"),
                    line_start: 1,
                    line_end: None,
                    snippet: String::new(),
                },
                Evidence {
                    path: PathBuf::from("src/second.rs"),
                    line_start: 2,
                    line_end: None,
                    snippet: String::new(),
                },
            ],
            ..Finding::default()
        };

        assert_eq!(matching_suppression_index(&finding, &index), Some(1));
    }
}
