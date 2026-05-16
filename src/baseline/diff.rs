use crate::baseline::key::stable_finding_key;
use crate::baseline::model::Baseline;
use crate::findings::types::Finding;
use crate::risk::apply_baseline_overlay;
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
}

pub fn diff_summary_against_baseline(
    mut summary: ScanSummary,
    baseline: &Baseline,
    baseline_path: PathBuf,
) -> BaselineScanReport {
    let baseline_keys = baseline
        .findings
        .iter()
        .map(|finding| finding.key.clone())
        .collect::<HashSet<_>>();

    let mut findings = status_findings(&summary, &baseline_keys);
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
mod tests {
    use super::*;
    use crate::baseline::model::{
        BASELINE_SCHEMA_VERSION, BASELINE_TOOL, Baseline, BaselineFinding,
    };
    use crate::findings::types::{Evidence, Finding, Severity};
    use crate::scan::types::ScanSummary;
    use std::path::PathBuf;

    fn make_finding(rule_id: &str, path: &str, line: usize) -> Finding {
        Finding {
            id: format!("{rule_id}-test"),
            rule_id: rule_id.to_string(),
            title: "Test".to_string(),
            severity: Severity::High,
            evidence: vec![Evidence {
                path: PathBuf::from(path),
                line_start: line,
                line_end: None,
                snippet: String::new(),
            }],
            ..Default::default()
        }
    }

    fn make_summary(findings: Vec<Finding>) -> ScanSummary {
        ScanSummary {
            root_path: PathBuf::from("/project"),
            findings,
            ..Default::default()
        }
    }

    fn baseline_with_keys(keys: Vec<String>) -> Baseline {
        Baseline {
            schema_version: BASELINE_SCHEMA_VERSION,
            tool: BASELINE_TOOL.to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            root: "/project".to_string(),
            findings: keys
                .into_iter()
                .map(|key| BaselineFinding {
                    key: key.clone(),
                    rule_id: "test.rule".to_string(),
                    severity: "HIGH".to_string(),
                    path: "src/main.rs".to_string(),
                    message: "Test".to_string(),
                })
                .collect(),
        }
    }

    #[test]
    fn new_finding_not_in_baseline_is_marked_new() {
        let finding = make_finding("test.rule", "src/main.rs", 10);
        let summary = make_summary(vec![finding]);
        let baseline = baseline_with_keys(vec![]);
        let baseline_path = PathBuf::from(".repopilot/baseline.json");

        let report = diff_summary_against_baseline(summary, &baseline, baseline_path);

        assert_eq!(report.findings[0].status, BaselineStatus::New);
        assert_eq!(report.new_count(), 1);
        assert_eq!(report.existing_count(), 0);
    }

    #[test]
    fn finding_in_baseline_is_marked_existing() {
        let finding = make_finding("test.rule", "src/main.rs", 10);
        let root = PathBuf::from("/project");
        let key = stable_finding_key(&finding, &root);
        let summary = make_summary(vec![finding]);
        let baseline = baseline_with_keys(vec![key]);
        let baseline_path = PathBuf::from(".repopilot/baseline.json");

        let report = diff_summary_against_baseline(summary, &baseline, baseline_path);

        assert_eq!(report.findings[0].status, BaselineStatus::Existing);
        assert_eq!(report.new_count(), 0);
        assert_eq!(report.existing_count(), 1);
    }

    #[test]
    fn all_findings_new_marks_every_finding_as_new() {
        let summary = make_summary(vec![
            make_finding("rule.one", "src/a.rs", 1),
            make_finding("rule.two", "src/b.rs", 2),
        ]);

        let report = all_findings_new(summary);

        assert_eq!(report.new_count(), 2);
        assert_eq!(report.existing_count(), 0);
        assert!(report.baseline_path.is_none());
    }

    #[test]
    fn mixed_new_and_existing_findings() {
        let new_finding = make_finding("rule.new", "src/new.rs", 5);
        let existing_finding = make_finding("rule.existing", "src/existing.rs", 10);
        let root = PathBuf::from("/project");
        let existing_key = stable_finding_key(&existing_finding, &root);

        let summary = make_summary(vec![new_finding, existing_finding]);
        let baseline = baseline_with_keys(vec![existing_key]);

        let report = diff_summary_against_baseline(
            summary,
            &baseline,
            PathBuf::from(".repopilot/baseline.json"),
        );

        assert_eq!(report.new_count(), 1);
        assert_eq!(report.existing_count(), 1);
    }
}
