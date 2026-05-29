use crate::findings::quality::SignalQualitySummary;
use crate::findings::types::Finding;
use crate::frameworks::DetectedFramework;
use crate::frameworks::FrameworkProject;
use crate::frameworks::ReactNativeArchitectureProfile;
use crate::risk::RiskPriority;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

mod support;

pub use support::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanMetadata {
    pub root_path: PathBuf,
    #[serde(default)]
    pub mode: ScanMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_ref: Option<String>,
    #[serde(default = "default_repo_level_rules_included")]
    pub repo_level_rules_included: bool,
    #[serde(default)]
    pub scan_duration_us: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scan_timings: Option<ScanTimings>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_telemetry: Option<ScanCacheTelemetry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_feedback: Option<LocalFeedbackReport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visibility_profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repopilotignore_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ScanMetrics {
    #[serde(default)]
    pub files_discovered: usize,
    pub files_analyzed: usize,
    pub directories_count: usize,
    pub non_empty_lines: usize,
    #[serde(default)]
    pub large_files_skipped: usize,
    #[serde(default)]
    pub files_skipped_low_signal: usize,
    #[serde(default)]
    pub binary_files_skipped: usize,
    #[serde(default)]
    pub skipped_bytes: u64,
    #[serde(default)]
    pub files_skipped_by_limit: usize,
    #[serde(default)]
    pub files_skipped_repopilotignore: usize,
    #[serde(default)]
    pub changed_files_count: usize,
    #[serde(default)]
    pub health_score: u8,
    #[serde(default)]
    pub raw_findings_count: usize,
    #[serde(default)]
    pub visible_findings_count: usize,
    #[serde(default)]
    pub hidden_suggestions_count: usize,
    pub languages: Vec<LanguageSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ScanArtifacts {
    pub findings: Vec<Finding>,
    #[serde(default)]
    pub detected_frameworks: Vec<DetectedFramework>,
    #[serde(default)]
    pub framework_projects: Vec<FrameworkProject>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub react_native: Option<ReactNativeArchitectureProfile>,
    #[serde(default)]
    pub coupling_graph: Option<CouplingGraph>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_graph_summary: Option<ContextGraphSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_graph_cache: Option<ContextGraphCacheInfo>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hidden_suggestions: Vec<HiddenSuggestionSummary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<ScanDiagnostic>,
    #[serde(default)]
    pub raw_signal_quality: SignalQualitySummary,
    #[serde(default)]
    pub visible_signal_quality: SignalQualitySummary,
    #[serde(default)]
    pub signal_quality: SignalQualitySummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ScanSummary {
    #[serde(flatten)]
    pub metadata: ScanMetadata,
    #[serde(flatten)]
    pub metrics: ScanMetrics,
    #[serde(flatten)]
    pub artifacts: ScanArtifacts,
}

fn default_repo_level_rules_included() -> bool {
    true
}

impl Default for ScanMetadata {
    fn default() -> Self {
        Self {
            root_path: PathBuf::new(),
            mode: ScanMode::Full,
            base_ref: None,
            repo_level_rules_included: true,
            scan_duration_us: 0,
            scan_timings: None,
            cache_telemetry: None,
            local_feedback: None,
            visibility_profile: None,
            repopilotignore_path: None,
        }
    }
}

impl ScanSummary {
    pub fn has_error_diagnostics(&self) -> bool {
        self.artifacts
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
    }

    pub fn first_error_diagnostic(&self) -> Option<&ScanDiagnostic> {
        self.artifacts
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
    }

    /// Computes the health score from findings and non-empty source lines.
    /// Penalty per finding type is normalized by project size (kloc) so large repos
    /// aren't unfairly penalized for having proportionally the same issue density.
    pub fn compute_health_score(findings: &[Finding], non_empty_lines: usize) -> u8 {
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
        let kloc = (non_empty_lines as f64 / 1000.0).max(0.5);
        let score = 100.0 - (penalty / kloc);
        score.clamp(0.0, 100.0) as u8
    }
}

impl std::ops::Deref for ScanSummary {
    type Target = ScanMetadata;
    fn deref(&self) -> &Self::Target {
        &self.metadata
    }
}

impl std::ops::DerefMut for ScanSummary {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.metadata
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LanguageSummary {
    pub name: String,
    pub files_analyzed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalSuppression {
    pub index: usize,
    pub rule_id: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalFeedbackReport {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feedback_path: Option<PathBuf>,
    pub suppressions_loaded: usize,
    pub suppressed_findings_count: usize,
    pub unmatched_suppressions_count: usize,
    pub invalid_suppressions_count: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unmatched_suppressions: Vec<LocalSuppression>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parse_error: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextGraphSummary {
    pub files: usize,
    pub import_edges: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub top_hubs: Vec<ContextGraphFileMetric>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub top_dependencies: Vec<ContextGraphFileMetric>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cycles: Vec<Vec<PathBuf>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changed_blast_radius: Vec<PathBuf>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub risky_clusters: Vec<ContextRiskCluster>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub truncated: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextGraphFileMetric {
    pub path: PathBuf,
    pub fan_in: usize,
    pub fan_out: usize,
    pub instability: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<String>,
}

impl Eq for ContextGraphFileMetric {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextRiskCluster {
    pub rule_id: String,
    pub scope: String,
    pub count: usize,
    pub max_score: u8,
    pub priority: RiskPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextGraphCacheInfo {
    pub status: String,
    pub reason: String,
    pub cache_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CouplingGraph {
    /// Outgoing edges: source file → set of files it imports.
    pub edges: BTreeMap<PathBuf, BTreeSet<PathBuf>>,
    /// Every scanned file, including those with no edges.
    pub nodes: BTreeSet<PathBuf>,
}

#[cfg(test)]
mod tests;
