use crate::findings::types::Finding;
use crate::frameworks::DetectedFramework;
use crate::frameworks::FrameworkProject;
use crate::frameworks::ReactNativeArchitectureProfile;
use crate::graph::CouplingGraph;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq)]
pub enum MarkerKind {
    Todo,
    Fixme,
    Hack,
}

#[derive(Debug)]
pub struct Marker {
    pub kind: MarkerKind,
    pub line_number: usize,
    pub path: PathBuf,
    pub text: String,
}

/// Per-phase wall-clock timings captured during a scan.
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanTimings {
    /// Time spent walking the file tree and running per-file audits (microseconds).
    pub file_scan_us: u64,
    /// Time spent detecting frameworks (microseconds).
    pub framework_detection_us: u64,
    /// Time spent running project, framework, and coupling audits (microseconds).
    pub post_scan_audits_us: u64,
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanSummary {
    pub root_path: PathBuf,
    /// Files found after gitignore, `.repopilotignore`, built-in ignores, and
    /// `--exclude` path/name filters are applied.
    #[serde(default)]
    pub files_discovered: usize,
    /// Text files actually analyzed. Skipped large, binary, low-signal, and
    /// `--max-files` capped files are not included.
    pub files_count: usize,
    pub directories_count: usize,
    pub lines_of_code: usize,
    #[serde(default)]
    pub skipped_files_count: usize,
    #[serde(default)]
    pub files_skipped_low_signal: usize,
    #[serde(default)]
    pub binary_files_skipped: usize,
    #[serde(default)]
    pub skipped_bytes: u64,
    pub languages: Vec<LanguageSummary>,
    pub findings: Vec<Finding>,
    #[serde(default)]
    pub detected_frameworks: Vec<DetectedFramework>,
    #[serde(default)]
    pub framework_projects: Vec<FrameworkProject>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub react_native: Option<ReactNativeArchitectureProfile>,
    #[serde(default)]
    pub coupling_graph: Option<CouplingGraph>,
    #[serde(default)]
    pub scan_duration_us: u64,
    /// 0–100 health score: 100 = no findings, decreases with severity-weighted finding density.
    #[serde(default)]
    pub health_score: u8,
    #[serde(default)]
    pub files_skipped_by_limit: usize,

    #[serde(default)]
    pub files_skipped_repopilotignore: usize,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repopilotignore_path: Option<PathBuf>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scan_timings: Option<ScanTimings>,
}

impl ScanSummary {
    /// Computes the health score from findings and lines of code.
    /// Penalty per finding type is normalized by project size (kloc) so large repos
    /// aren't unfairly penalized for having proportionally the same issue density.
    pub fn compute_health_score(findings: &[Finding], lines_of_code: usize) -> u8 {
        use crate::findings::types::Severity;
        let mut penalty = 0.0f64;
        for f in findings {
            penalty += match f.severity {
                Severity::Critical => 20.0,
                Severity::High => 5.0,
                Severity::Medium => 2.0,
                Severity::Low => 0.5,
                Severity::Info => 0.0,
            };
        }
        let kloc = (lines_of_code as f64 / 1000.0).max(0.5);
        let score = 100.0 - (penalty / kloc);
        score.clamp(0.0, 100.0) as u8
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LanguageSummary {
    pub name: String,
    pub files_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::findings::types::{Finding, Severity};

    fn finding_with_severity(severity: Severity) -> Finding {
        Finding {
            severity,
            ..Default::default()
        }
    }

    #[test]
    fn health_score_is_100_for_no_findings() {
        let score = ScanSummary::compute_health_score(&[], 10_000);
        assert_eq!(score, 100);
    }

    #[test]
    fn health_score_degrades_with_critical_findings() {
        let findings = vec![finding_with_severity(Severity::Critical)];
        let score = ScanSummary::compute_health_score(&findings, 10_000);
        assert!(score < 100);
    }

    #[test]
    fn health_score_is_clamped_to_zero() {
        let findings: Vec<Finding> = (0..50)
            .map(|_| finding_with_severity(Severity::Critical))
            .collect();
        let score = ScanSummary::compute_health_score(&findings, 1_000);
        assert_eq!(score, 0);
    }

    #[test]
    fn health_score_same_findings_higher_for_larger_codebase() {
        let findings = vec![
            finding_with_severity(Severity::High),
            finding_with_severity(Severity::High),
        ];
        let score_small = ScanSummary::compute_health_score(&findings, 2_000);
        let score_large = ScanSummary::compute_health_score(&findings, 100_000);
        assert!(score_large > score_small);
    }

    #[test]
    fn info_findings_do_not_reduce_score() {
        let findings = vec![
            finding_with_severity(Severity::Info),
            finding_with_severity(Severity::Info),
        ];
        let score = ScanSummary::compute_health_score(&findings, 10_000);
        assert_eq!(score, 100);
    }
}
