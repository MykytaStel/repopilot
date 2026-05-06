use crate::audits::traits::FileAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::scan::config::ScanConfig;
use crate::scan::facts::FileFacts;

pub struct ComplexityAudit;

impl FileAudit for ComplexityAudit {
    fn audit(&self, file: &FileFacts, config: &ScanConfig) -> Vec<Finding> {
        if file.content.is_empty() || file.lines_of_code < 10 {
            return vec![];
        }
        if !is_code_language(file.language.as_deref()) {
            return vec![];
        }

        let branch_count = count_branches(&file.content);
        let density = branch_count.saturating_mul(1000) / file.lines_of_code;

        let severity = if density >= config.complexity_high_threshold {
            Severity::High
        } else if density >= config.complexity_medium_threshold {
            Severity::Medium
        } else {
            return vec![];
        };

        let threshold = if severity == Severity::High {
            config.complexity_high_threshold
        } else {
            config.complexity_medium_threshold
        };

        vec![Finding {
            id: String::new(),
            rule_id: "code-quality.complex-file".to_string(),
            title: "High complexity density".to_string(),
            description: format!(
                "This file has a complexity density of {density} (branch constructs × 1000 / LOC), \
                 above the {threshold} threshold. High density often indicates tangled logic — \
                 consider extracting helpers or splitting responsibilities."
            ),
            category: FindingCategory::CodeQuality,
            severity,
            evidence: vec![Evidence {
                path: file.path.clone(),
                line_start: 1,
                line_end: None,
                snippet: format!(
                    "branch_count={branch_count}, lines_of_code={}, density={density}",
                    file.lines_of_code
                ),
            }],
        }]
    }
}

fn is_code_language(language: Option<&str>) -> bool {
    matches!(
        language,
        Some(
            "Rust"
                | "Go"
                | "Python"
                | "TypeScript"
                | "TypeScript React"
                | "JavaScript"
                | "JavaScript React"
                | "Java"
                | "Kotlin"
                | "C"
                | "C++"
        )
    )
}

/// Counts branching constructs and logical operators as a heuristic complexity metric.
/// Skips comment-only lines. Word-boundary check prevents matching inside identifiers.
pub fn count_branches(content: &str) -> usize {
    const KEYWORDS: &[&str] = &[
        "if ", "elif ", "for ", "while ", "match ", "switch ", "case ", "catch ",
    ];
    const OPERATORS: &[&str] = &["&&", "||"];

    let mut count = 0usize;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("//")
            || trimmed.starts_with('#')
            || trimmed.starts_with('*')
            || trimmed.starts_with("/*")
        {
            continue;
        }

        for op in OPERATORS {
            let mut rest = trimmed;
            while let Some(pos) = rest.find(op) {
                count += 1;
                rest = &rest[pos + op.len()..];
            }
        }

        for kw in KEYWORDS {
            let mut rest = trimmed;
            let mut offset = 0usize;
            while let Some(pos) = rest.find(kw) {
                let abs = offset + pos;
                let prev_char = trimmed.as_bytes().get(abs.wrapping_sub(1));
                let good_boundary =
                    abs == 0 || prev_char.is_none_or(|b| !b.is_ascii_alphanumeric() && *b != b'_');
                if good_boundary {
                    count += 1;
                }
                let step = pos + 1;
                rest = &rest[step..];
                offset += step;
            }
        }
    }

    count
}
