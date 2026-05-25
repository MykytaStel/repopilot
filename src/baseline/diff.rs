use crate::baseline::key::{normalized_relative_path, stable_finding_key};
use crate::baseline::model::{Baseline, BaselineFinding};
use crate::findings::filter::{FindingFilter, recompute_summary_metrics};
use crate::findings::types::Finding;
use crate::risk::apply_baseline_overlay;
use crate::scan::types::ScanSummary;
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BaselineStatus {
    New,
    Existing,
}

impl BaselineStatus {
    pub fn lowercase_label(self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Existing => "existing",
        }
    }
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

    pub fn apply_filter(&mut self, filter: &FindingFilter) {
        self.retain_findings(|finding| filter.matches(finding));
    }

    pub fn retain_findings<F>(&mut self, mut keep: F)
    where
        F: FnMut(&Finding) -> bool,
    {
        let mut paired = self
            .summary
            .findings
            .drain(..)
            .zip(self.findings.drain(..))
            .collect::<Vec<_>>();

        paired.retain(|(finding, _)| keep(finding));

        for (finding, status) in paired {
            self.summary.findings.push(finding);
            self.findings.push(status);
        }

        recompute_summary_metrics(&mut self.summary);
    }
}

pub fn diff_summary_against_baseline(
    mut summary: ScanSummary,
    baseline: &Baseline,
    baseline_path: PathBuf,
) -> BaselineScanReport {
    let baseline_index = BaselineIndex::from_baseline(baseline);

    let mut findings = status_findings(&summary, &baseline_index);
    apply_baseline_overlay(
        &mut summary.findings,
        &findings,
        summary.root_path.as_path(),
    );
    sort_findings_with_status(&mut summary.findings, &mut findings);

    BaselineScanReport {
        summary,
        baseline_path: Some(baseline_path),
        findings,
    }
}

pub fn all_findings_new(mut summary: ScanSummary) -> BaselineScanReport {
    let mut findings: Vec<FindingBaselineStatus> = summary
        .findings
        .iter()
        .map(|finding| FindingBaselineStatus {
            key: stable_finding_key(finding, &summary.root_path),
            status: BaselineStatus::New,
        })
        .collect();
    apply_baseline_overlay(
        &mut summary.findings,
        &findings,
        summary.root_path.as_path(),
    );
    sort_findings_with_status(&mut summary.findings, &mut findings);

    BaselineScanReport {
        summary,
        baseline_path: None,
        findings,
    }
}

fn status_findings(
    summary: &ScanSummary,
    baseline_index: &BaselineIndex,
) -> Vec<FindingBaselineStatus> {
    let root = summary.root_path.as_path();

    summary
        .findings
        .iter()
        .map(|finding| {
            let key = stable_finding_key(finding, root);
            let status = if baseline_index.contains_finding(finding, root, &key) {
                BaselineStatus::Existing
            } else {
                BaselineStatus::New
            };

            FindingBaselineStatus { key, status }
        })
        .collect()
}

struct BaselineIndex {
    keys: HashSet<String>,
    legacy_descriptors: HashSet<BaselineDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct BaselineDescriptor {
    rule_id: String,
    path: String,
    message: String,
}

impl BaselineIndex {
    fn from_baseline(baseline: &Baseline) -> Self {
        let keys = baseline
            .findings
            .iter()
            .map(|finding| finding.key.clone())
            .collect::<HashSet<_>>();
        let legacy_descriptors = baseline
            .findings
            .iter()
            .filter(|finding| is_legacy_line_key(&finding.key))
            .map(BaselineDescriptor::from_baseline_finding)
            .collect::<HashSet<_>>();

        Self {
            keys,
            legacy_descriptors,
        }
    }

    fn contains_finding(&self, finding: &Finding, root: &Path, key: &str) -> bool {
        self.keys.contains(key)
            || self
                .legacy_descriptors
                .contains(&BaselineDescriptor::from_finding(finding, root))
    }
}

impl BaselineDescriptor {
    fn from_baseline_finding(finding: &BaselineFinding) -> Self {
        Self {
            rule_id: finding.rule_id.clone(),
            path: clean_baseline_path(&finding.path),
            message: finding.message.clone(),
        }
    }

    fn from_finding(finding: &Finding, root: &Path) -> Self {
        let path = finding
            .evidence
            .first()
            .map(|evidence| normalized_relative_path(&evidence.path, root))
            .unwrap_or_else(|| ".".to_string());

        Self {
            rule_id: finding.rule_id.clone(),
            path: clean_baseline_path(&path),
            message: finding.title.clone(),
        }
    }
}

fn is_legacy_line_key(key: &str) -> bool {
    let Some((_, suffix)) = key.rsplit_once(':') else {
        return false;
    };

    if let Some((start, end)) = suffix.split_once('-') {
        return parse_positive_usize(start) && parse_positive_usize(end);
    }

    parse_positive_usize(suffix)
}

fn parse_positive_usize(value: &str) -> bool {
    value.parse::<usize>().is_ok_and(|parsed| parsed > 0)
}

fn clean_baseline_path(path: &str) -> String {
    let path = path.replace('\\', "/");
    let path = path.trim_start_matches("./");

    if path.is_empty() {
        ".".to_string()
    } else {
        path.to_string()
    }
}

fn sort_findings_with_status(
    findings: &mut Vec<Finding>,
    statuses: &mut Vec<FindingBaselineStatus>,
) {
    let mut paired = findings
        .drain(..)
        .zip(statuses.drain(..))
        .collect::<Vec<_>>();
    paired.sort_by(|(left, _), (right, _)| crate::risk::compare_findings(left, right));
    for (finding, status) in paired {
        findings.push(finding);
        statuses.push(status);
    }
}

#[cfg(test)]
mod tests;
