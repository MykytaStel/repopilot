use crate::baseline::key::stable_finding_key;
use crate::baseline::model::Baseline;
use crate::findings::types::Finding;
use crate::scan::types::ScanSummary;
use serde::Serialize;
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BaselineStatus {
    New,
    Existing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FindingBaselineStatus {
    pub key: String,
    pub status: BaselineStatus,
}

#[derive(Debug, PartialEq, Eq)]
pub struct BaselineScanReport {
    pub summary: ScanSummary,
    pub baseline_path: Option<PathBuf>,
    pub findings: Vec<FindingBaselineStatus>,
}

impl BaselineScanReport {
    pub fn new_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.status == BaselineStatus::New)
            .count()
    }

    pub fn existing_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.status == BaselineStatus::Existing)
            .count()
    }

    pub fn finding_status(&self, index: usize) -> BaselineStatus {
        self.findings
            .get(index)
            .map(|finding| finding.status)
            .unwrap_or(BaselineStatus::New)
    }

    pub fn findings_with_status(&self, status: BaselineStatus) -> Vec<&Finding> {
        self.summary
            .findings
            .iter()
            .enumerate()
            .filter_map(|(index, finding)| {
                (self.finding_status(index) == status).then_some(finding)
            })
            .collect()
    }
}

pub fn diff_summary_against_baseline(
    summary: ScanSummary,
    baseline: &Baseline,
    baseline_path: PathBuf,
) -> BaselineScanReport {
    let baseline_keys = baseline
        .findings
        .iter()
        .map(|finding| finding.key.clone())
        .collect::<HashSet<_>>();

    let findings = status_findings(&summary, &baseline_keys);

    BaselineScanReport {
        summary,
        baseline_path: Some(baseline_path),
        findings,
    }
}

pub fn all_findings_new(summary: ScanSummary) -> BaselineScanReport {
    let findings = summary
        .findings
        .iter()
        .map(|finding| FindingBaselineStatus {
            key: stable_finding_key(finding, &summary.root_path),
            status: BaselineStatus::New,
        })
        .collect();

    BaselineScanReport {
        summary,
        baseline_path: None,
        findings,
    }
}

fn status_findings(
    summary: &ScanSummary,
    baseline_keys: &HashSet<String>,
) -> Vec<FindingBaselineStatus> {
    let root = summary.root_path.as_path();

    summary
        .findings
        .iter()
        .map(|finding| {
            let key = stable_finding_key(finding, root);
            let status = if baseline_keys.contains(&key) {
                BaselineStatus::Existing
            } else {
                BaselineStatus::New
            };

            FindingBaselineStatus { key, status }
        })
        .collect()
}
