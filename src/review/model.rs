use crate::baseline::diff::BaselineStatus;
use crate::findings::filter::{FindingFilter, recompute_summary_metrics};
use crate::findings::types::{Finding, Severity};
use crate::review::diff::{ChangeStatus, ChangedFile};
use crate::review::impact::ImpactPaths;
use crate::review::signals::BoundarySignal;
use crate::review::signals::tiered::TieredSignals;
use crate::scan::types::ScanSummary;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct ReviewTimings {
    pub diff_loading_us: u64,
    pub review_signals_us: u64,
    pub gating_us: u64,
    pub rendering_us: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ReviewReport {
    pub summary: ScanSummary,
    pub repo_root: PathBuf,
    pub baseline_path: Option<PathBuf>,
    pub changed_files: Vec<ChangedFile>,
    pub blast_radius: Vec<PathBuf>,
    pub impact_paths: ImpactPaths,
    pub boundary_signals: Vec<BoundarySignal>,
    /// A code boundary (auth / request-trust) changed but no test moved.
    pub boundary_missing_test: bool,
    /// Boundary + behavioral + algorithmic + taint signals by confidence tier.
    pub tiered_signals: TieredSignals,
    pub timings: ReviewTimings,
    pub findings: Vec<ReviewFindingStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReviewFindingStatus {
    pub key: String,
    pub in_diff: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baseline_status: Option<BaselineStatus>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct SeverityCounts {
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub info: usize,
}

impl ReviewReport {
    pub fn in_diff_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.in_diff)
            .count()
    }

    pub fn out_of_diff_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|finding| !finding.in_diff)
            .count()
    }

    pub fn new_in_diff_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|finding| {
                finding.in_diff && finding.baseline_status == Some(BaselineStatus::New)
            })
            .count()
    }

    pub fn existing_in_diff_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|finding| {
                finding.in_diff && finding.baseline_status == Some(BaselineStatus::Existing)
            })
            .count()
    }

    pub fn deleted_files(&self) -> Vec<&ChangedFile> {
        self.changed_files
            .iter()
            .filter(|file| file.status == ChangeStatus::Deleted)
            .collect()
    }

    pub fn in_diff_findings(&self) -> Vec<&Finding> {
        self.summary
            .artifacts
            .findings
            .iter()
            .enumerate()
            .filter_map(|(index, finding)| {
                self.findings
                    .get(index)
                    .and_then(|status| status.in_diff.then_some(finding))
            })
            .collect()
    }

    pub fn out_of_diff_findings(&self) -> Vec<&Finding> {
        self.summary
            .artifacts
            .findings
            .iter()
            .enumerate()
            .filter_map(|(index, finding)| {
                self.findings
                    .get(index)
                    .and_then(|status| (!status.in_diff).then_some(finding))
            })
            .collect()
    }

    pub fn severity_counts(&self) -> SeverityCounts {
        let mut counts = SeverityCounts::default();

        for finding in self.in_diff_findings() {
            counts.add(finding.severity);
        }

        counts
    }

    pub fn finding_status(&self, index: usize) -> Option<&ReviewFindingStatus> {
        self.findings.get(index)
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
            .artifacts
            .findings
            .drain(..)
            .zip(self.findings.drain(..))
            .collect::<Vec<_>>();

        paired.retain(|(finding, _)| keep(finding));

        for (finding, status) in paired {
            self.summary.artifacts.findings.push(finding);
            self.findings.push(status);
        }

        recompute_summary_metrics(&mut self.summary);
    }

    pub fn retain_in_diff_findings(&mut self) {
        let mut paired = self
            .summary
            .artifacts
            .findings
            .drain(..)
            .zip(self.findings.drain(..))
            .collect::<Vec<_>>();
        paired.retain(|(_, status)| status.in_diff);
        for (finding, status) in paired {
            self.summary.artifacts.findings.push(finding);
            self.findings.push(status);
        }
        recompute_summary_metrics(&mut self.summary);
    }
}

impl SeverityCounts {
    fn add(&mut self, severity: Severity) {
        match severity {
            Severity::Critical => self.critical += 1,
            Severity::High => self.high += 1,
            Severity::Medium => self.medium += 1,
            Severity::Low => self.low += 1,
            Severity::Info => self.info += 1,
        }
    }
}
