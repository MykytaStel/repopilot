use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Finding {
    pub id: String,
    pub rule_id: String,
    pub title: String,
    pub description: String,
    pub category: FindingCategory,
    pub severity: Severity,
    pub evidence: Vec<Evidence>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FindingCategory {
    Architecture,
    CodeQuality,
    Testing,
    Security,
    Performance,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Evidence {
    pub path: PathBuf,
    pub line_start: usize,
    pub line_end: Option<usize>,
    pub snippet: String,
}

impl Finding {
    pub fn severity_label(&self) -> &'static str {
        self.severity.label()
    }
}

impl Severity {
    pub fn label(&self) -> &'static str {
        match self {
            Severity::Info => "INFO",
            Severity::Low => "LOW",
            Severity::Medium => "MEDIUM",
            Severity::High => "HIGH",
            Severity::Critical => "CRITICAL",
        }
    }
}
