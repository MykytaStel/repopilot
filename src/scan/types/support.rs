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
    /// Time spent validating the internal finding contract.
    #[serde(default)]
    pub contract_validation_us: u64,
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
            .saturating_add(self.contract_validation_us)
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

pub fn cache_diagnostic(error: &std::io::Error) -> ScanDiagnostic {
    ScanDiagnostic::warning(
        "context-graph.cache-write-failed",
        format!("Could not write context graph cache: {error}"),
    )
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
