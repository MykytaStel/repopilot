use crate::audits::traits::FileAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::scan::config::ScanConfig;
use crate::scan::facts::FileFacts;
use crate::scan::markers::detect_markers;
use crate::scan::types::{Marker, MarkerKind};

pub struct CodeMarkerAudit;

impl FileAudit for CodeMarkerAudit {
    fn audit(&self, file: &FileFacts, _config: &ScanConfig) -> Vec<Finding> {
        if should_skip_marker_audit(&file.path) {
            return vec![];
        }

        detect_markers(&file.path, &file.content)
            .iter()
            .map(build_marker_finding)
            .collect()
    }
}

pub fn detect_code_marker_findings(file: &FileFacts) -> Vec<Finding> {
    detect_markers(&file.path, &file.content)
        .iter()
        .map(build_marker_finding)
        .collect()
}

fn build_marker_finding(marker: &Marker) -> Finding {
    let marker_str = match marker.kind {
        MarkerKind::Todo => "todo",
        MarkerKind::Fixme => "fixme",
        MarkerKind::Hack => "hack",
    };
    let uppercase = marker_str.to_uppercase();

    Finding {
        id: format!(
            "code-marker.{}.{}:{}",
            marker_str,
            marker.path.display(),
            marker.line_number
        ),
        rule_id: format!("code-marker.{marker_str}"),
        title: format!("{uppercase} marker found"),
        description: format!(
            "A {uppercase} marker was found in the codebase and should be reviewed."
        ),
        category: FindingCategory::CodeQuality,
        severity: marker_severity(marker_str),
        evidence: vec![Evidence {
            path: marker.path.clone(),
            line_start: marker.line_number,
            line_end: None,
            snippet: marker.text.trim().to_string(),
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

fn should_skip_marker_audit(path: &std::path::Path) -> bool {
    let is_markdown = path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"));

    is_markdown || has_component(path, "tests") || has_component(path, "test")
}

fn has_component(path: &std::path::Path, component: &str) -> bool {
    path.components()
        .any(|c| c.as_os_str().to_string_lossy() == component)
}
