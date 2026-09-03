use crate::audits::context::classify_file;
use crate::audits::traits::FileAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::knowledge::decision::{decide_for_audit_context, record_decision_provenance};
use crate::scan::config::ScanConfig;
use crate::scan::facts::FileFacts;

pub struct ComplexityAudit;

const RULE_ID: &str = "code-quality.complex-file";
const MIN_HIGH_COMPLEXITY_LOC: usize = 25;

impl FileAudit for ComplexityAudit {
    fn audit(&self, file: &FileFacts, config: &ScanConfig) -> Vec<Finding> {
        if file.non_empty_lines < 10 {
            return vec![];
        }

        let density = file.branch_count.saturating_mul(1000) / file.non_empty_lines;

        let base_severity = if density >= config.complexity_high_threshold
            && file.non_empty_lines >= MIN_HIGH_COMPLEXITY_LOC
        {
            Severity::High
        } else if density >= config.complexity_medium_threshold {
            Severity::Medium
        } else {
            return vec![];
        };

        let context = classify_file(file);
        let decision = decide_for_audit_context(RULE_ID, &context, base_severity, None);

        if decision.is_suppressed() {
            return vec![];
        }

        let severity = decision.severity;

        let threshold = if severity == Severity::High {
            config.complexity_high_threshold
        } else {
            config.complexity_medium_threshold
        };

        let mut finding = Finding {
            id: String::new(),
            rule_id: RULE_ID.to_string(),
            recommendation: Finding::recommendation_for_rule_id(RULE_ID),
            title: "High complexity density".to_string(),
            description: format!(
                "This file has a complexity density of {density} (branch constructs × 1000 / LOC), \
                 above the {threshold} threshold. High density often indicates tangled logic — \
                 consider extracting helpers or splitting responsibilities."
            ),
            category: FindingCategory::CodeQuality,
            severity,
            confidence: Default::default(),
            evidence: vec![Evidence {
                path: file.path.clone(),
                line_start: 1,
                line_end: None,
                snippet: format!(
                    "branch_count={}, non_empty_lines={}, density={density}",
                    file.branch_count, file.non_empty_lines
                ),
            }],
            workspace_package: None,
            docs_url: None,
            provenance: Default::default(),
            risk: Default::default(),
        };
        record_decision_provenance(&mut finding, base_severity, None, &decision);
        vec![finding]
    }
}

/// Counts branching constructs and logical operators as a heuristic complexity metric.
/// Skips comment-only lines. Word-boundary check prevents matching inside identifiers.
pub fn count_branches(content: &str) -> usize {
    content.lines().map(count_line_branches).sum()
}

fn count_line_branches(line: &str) -> usize {
    let trimmed = line.trim();
    if is_comment_line(trimmed) {
        return 0;
    }

    count_operator_matches(trimmed) + count_keyword_matches(trimmed)
}

fn is_comment_line(line: &str) -> bool {
    line.starts_with("//")
        || line.starts_with('#')
        || line.starts_with('*')
        || line.starts_with("/*")
}

fn count_operator_matches(line: &str) -> usize {
    ["&&", "||"]
        .iter()
        .map(|operator| count_substrings(line, operator))
        .sum()
}

fn count_keyword_matches(line: &str) -> usize {
    [
        "if ", "else ", "elif ", "for ", "while ", "match ", "switch ", "case ", "catch ",
    ]
    .iter()
    .map(|keyword| count_bounded_keyword(line, keyword))
    .sum()
}

fn count_substrings(line: &str, needle: &str) -> usize {
    let mut count = 0;
    let mut rest = line;
    while let Some(pos) = rest.find(needle) {
        count += 1;
        rest = &rest[pos + needle.len()..];
    }
    count
}

fn count_bounded_keyword(line: &str, keyword: &str) -> usize {
    let mut count = 0;
    let mut rest = line;
    let mut offset = 0usize;
    while let Some(pos) = rest.find(keyword) {
        let absolute = offset + pos;
        let previous = line.as_bytes().get(absolute.wrapping_sub(1));
        if absolute == 0
            || previous.is_none_or(|byte| !byte.is_ascii_alphanumeric() && *byte != b'_')
        {
            count += 1;
        }
        let step = pos + 1;
        rest = &rest[step..];
        offset += step;
    }
    count
}

#[cfg(test)]
mod tests {
    use super::count_line_branches;

    #[test]
    fn line_branch_counter_counts_keyword_and_logical_operator() {
        assert_eq!(count_line_branches("if ready && valid {"), 2);
    }
}
