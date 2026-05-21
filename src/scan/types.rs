use crate::findings::feedback::LocalFeedbackReport;
use crate::findings::quality::SignalQualitySummary;
use crate::findings::types::Finding;
use crate::frameworks::DetectedFramework;
use crate::frameworks::FrameworkProject;
use crate::frameworks::ReactNativeArchitectureProfile;
use crate::graph::CouplingGraph;
use crate::graph::context::{ContextGraphCacheInfo, ContextGraphSummary};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

mod support;

pub use support::*;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanSummary {
    pub root_path: PathBuf,
    #[serde(default)]
    pub mode: ScanMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_ref: Option<String>,
    #[serde(default)]
    pub changed_files_count: usize,
    #[serde(default = "default_repo_level_rules_included")]
    pub repo_level_rules_included: bool,
    /// Files found after gitignore, `.repopilotignore`, built-in ignores, and
    /// `--exclude` path/name filters are applied.
    #[serde(default)]
    pub files_discovered: usize,
    /// Text files actually analyzed. Skipped large, binary, low-signal, and
    /// `--max-files` capped files are not included.
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_graph_summary: Option<ContextGraphSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_graph_cache: Option<ContextGraphCacheInfo>,
    #[serde(default)]
    pub scan_duration_us: u64,
    /// 0–100 health score: 100 = no findings, decreases with severity-weighted finding density.
    #[serde(default)]
    pub health_score: u8,
    #[serde(default)]
    pub visible_findings_count: usize,
    #[serde(default)]
    pub hidden_suggestions_count: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hidden_suggestions: Vec<HiddenSuggestionSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visibility_profile: Option<String>,
    #[serde(default)]
    pub files_skipped_by_limit: usize,

    #[serde(default)]
    pub files_skipped_repopilotignore: usize,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repopilotignore_path: Option<PathBuf>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scan_timings: Option<ScanTimings>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_telemetry: Option<ScanCacheTelemetry>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_feedback: Option<LocalFeedbackReport>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<ScanDiagnostic>,

    #[serde(default)]
    pub signal_quality: SignalQualitySummary,
}

fn default_repo_level_rules_included() -> bool {
    true
}

impl Default for ScanSummary {
    fn default() -> Self {
        Self {
            root_path: PathBuf::new(),
            mode: ScanMode::Full,
            base_ref: None,
            changed_files_count: 0,
            repo_level_rules_included: true,
            files_discovered: 0,
            files_analyzed: 0,
            directories_count: 0,
            non_empty_lines: 0,
            large_files_skipped: 0,
            files_skipped_low_signal: 0,
            binary_files_skipped: 0,
            skipped_bytes: 0,
            languages: Vec::new(),
            findings: Vec::new(),
            detected_frameworks: Vec::new(),
            framework_projects: Vec::new(),
            react_native: None,
            coupling_graph: None,
            context_graph_summary: None,
            context_graph_cache: None,
            scan_duration_us: 0,
            health_score: 0,
            visible_findings_count: 0,
            hidden_suggestions_count: 0,
            hidden_suggestions: Vec::new(),
            visibility_profile: None,
            files_skipped_by_limit: 0,
            files_skipped_repopilotignore: 0,
            repopilotignore_path: None,
            scan_timings: None,
            cache_telemetry: None,
            local_feedback: None,
            diagnostics: Vec::new(),
            signal_quality: SignalQualitySummary::default(),
        }
    }
}

impl ScanSummary {
    pub fn has_error_diagnostics(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
    }

    pub fn first_error_diagnostic(&self) -> Option<&ScanDiagnostic> {
        self.diagnostics
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LanguageSummary {
    pub name: String,
    pub files_analyzed: usize,
}

#[cfg(test)]
mod tests;
