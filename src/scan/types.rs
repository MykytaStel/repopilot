use crate::findings::feedback::LocalFeedbackReport;
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
    /// Time spent discovering candidate files before analysis (microseconds).
    #[serde(default)]
    pub discovery_us: u64,
    /// Time spent loading files and running per-file audits (microseconds).
    #[serde(default)]
    pub file_analysis_us: u64,
    /// Time spent walking the file tree and running per-file audits (microseconds).
    pub file_scan_us: u64,
    /// Time spent detecting frameworks (microseconds).
    pub framework_detection_us: u64,
    /// Time spent running project, framework, and coupling audits (microseconds).
    pub post_scan_audits_us: u64,
    /// Time spent populating recommendations, IDs, and derived finding metadata.
    #[serde(default)]
    pub enrichment_us: u64,
    /// Time spent applying risk scoring and risk overlays.
    #[serde(default)]
    pub risk_scoring_us: u64,
    /// Time spent building the final report summary object.
    #[serde(default)]
    pub report_finalization_us: u64,
}

impl ScanTimings {
    pub fn accounted_engine_us(&self) -> u64 {
        let file_pipeline_us = self
            .file_scan_us
            .max(self.discovery_us.saturating_add(self.file_analysis_us));

        file_pipeline_us
            .saturating_add(self.framework_detection_us)
            .saturating_add(self.post_scan_audits_us)
            .saturating_add(self.enrichment_us)
            .saturating_add(self.risk_scoring_us)
            .saturating_add(self.report_finalization_us)
    }
}

/// Cache effectiveness and per-file cache decisions captured during changed scans.
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanCacheTelemetry {
    pub hits: usize,
    pub misses: usize,
    pub skipped: usize,
    pub hit_rate_percent: u8,
    pub changed_file_reasons: Vec<ChangedFileReasonSummary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changed_files: Vec<ChangedFileCacheTelemetry>,
    pub timings: ScanCacheTimings,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangedFileReasonSummary {
    pub reason: String,
    pub count: usize,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangedFileCacheTelemetry {
    pub path: PathBuf,
    pub change_reason: String,
    pub cache_status: String,
    pub cache_reason: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanCacheTimings {
    pub load_us: u64,
    pub file_hash_us: u64,
    pub lookup_us: u64,
    pub hit_reuse_us: u64,
    pub miss_scan_us: u64,
    pub write_us: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_time_saved_us: Option<u64>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HiddenSuggestionSummary {
    pub intent: String,
    pub rule_id: String,
    pub category: String,
    pub reason: String,
    pub count: usize,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticSeverity {
    #[default]
    Info,
    Warning,
    Error,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanDiagnostic {
    pub code: String,
    pub severity: DiagnosticSeverity,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
}

impl ScanDiagnostic {
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            severity: DiagnosticSeverity::Error,
            message: message.into(),
            path: None,
        }
    }

    pub fn warning(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            severity: DiagnosticSeverity::Warning,
            message: message.into(),
            path: None,
        }
    }

    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ScanMode {
    #[default]
    Full,
    Changed,
}

impl ScanMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Changed => "changed",
        }
    }
}

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
