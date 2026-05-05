use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use std::path::Path;

pub fn detect_marker_findings(path: &Path, content: &str) -> Vec<Finding> {
    let mut findings = Vec::new();

    for (index, line) in content.lines().enumerate() {
        if line.contains("TODO") {
            findings.push(build_marker_finding(path, index, line, "todo"));
        }

        if line.contains("FIXME") {
            findings.push(build_marker_finding(path, index, line, "fixme"));
        }

        if line.contains("HACK") {
            findings.push(build_marker_finding(path, index, line, "hack"));
        }
    }

    findings
}

fn build_marker_finding(path: &Path, index: usize, line: &str, marker: &str) -> Finding {
    let line_number = index + 1;
    let uppercase_marker = marker.to_uppercase();

    Finding {
        id: format!("code-marker.{}.{}:{}", marker, path.display(), line_number),
        rule_id: format!("code-marker.{marker}"),
        title: format!("{uppercase_marker} marker found"),
        description: format!(
            "A {uppercase_marker} marker was found in the codebase and should be reviewed."
        ),
        category: FindingCategory::CodeQuality,
        severity: marker_severity(marker),
        evidence: vec![Evidence {
            path: path.to_path_buf(),
            line_start: line_number,
            line_end: None,
            snippet: line.trim().to_string(),
        }],
    }
}

fn marker_severity(marker: &str) -> Severity {
    match marker {
        "fixme" => Severity::Medium,
        "hack" => Severity::Medium,
        "todo" => Severity::Low,
        _ => Severity::Info,
    }
}
