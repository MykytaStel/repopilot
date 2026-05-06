use crate::baseline::key::{normalized_relative_path, stable_finding_key};
use crate::findings::types::Finding;
use crate::scan::types::ScanSummary;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const BASELINE_SCHEMA_VERSION: u32 = 1;
pub const BASELINE_TOOL: &str = "repopilot";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Baseline {
    pub schema_version: u32,
    pub tool: String,
    pub created_at: String,
    pub root: String,
    pub findings: Vec<BaselineFinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BaselineFinding {
    pub key: String,
    pub rule_id: String,
    pub severity: String,
    pub path: String,
    pub message: String,
}

impl Baseline {
    pub fn from_scan_summary(
        summary: &ScanSummary,
        root: &Path,
        display_root: String,
        created_at: String,
    ) -> Self {
        let mut findings = summary
            .findings
            .iter()
            .map(|finding| BaselineFinding::from_finding(finding, root))
            .collect::<Vec<_>>();

        findings.sort_by(|left, right| left.key.cmp(&right.key));

        Self {
            schema_version: BASELINE_SCHEMA_VERSION,
            tool: BASELINE_TOOL.to_string(),
            created_at,
            root: display_root,
            findings,
        }
    }
}

impl BaselineFinding {
    pub fn from_finding(finding: &Finding, root: &Path) -> Self {
        let path = finding
            .evidence
            .first()
            .map(|evidence| normalized_relative_path(&evidence.path, root))
            .unwrap_or_else(|| ".".to_string());

        Self {
            key: stable_finding_key(finding, root),
            rule_id: finding.rule_id.clone(),
            severity: finding.severity.lowercase_label().to_string(),
            path,
            message: finding.title.clone(),
        }
    }
}
