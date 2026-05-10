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

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanSummary {
    pub root_path: PathBuf,
    pub files_count: usize,
    pub directories_count: usize,
    pub lines_of_code: usize,
    #[serde(default)]
    pub skipped_files_count: usize,
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
