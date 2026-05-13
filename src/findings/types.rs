use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Finding {
    pub id: String,
    pub rule_id: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub recommendation: String,
    pub category: FindingCategory,
    pub severity: Severity,
    #[serde(default)]
    pub confidence: Confidence,
    pub evidence: Vec<Evidence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_package: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub docs_url: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FindingCategory {
    #[default]
    Architecture,
    CodeQuality,
    Testing,
    Security,
    Framework,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Severity {
    #[default]
    Info,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Confidence {
    Low,
    #[default]
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Evidence {
    pub path: PathBuf,
    pub line_start: usize,
    pub line_end: Option<usize>,
    pub snippet: String,
}

impl FindingCategory {
    pub fn label(&self) -> &'static str {
        match self {
            FindingCategory::Architecture => "architecture",
            FindingCategory::CodeQuality => "code-quality",
            FindingCategory::Testing => "testing",
            FindingCategory::Security => "security",
            FindingCategory::Framework => "framework",
        }
    }
}

impl Finding {
    pub const GENERIC_RECOMMENDATION: &'static str = "Review the finding evidence, confirm the risk in context, and make the smallest safe change that addresses the underlying issue.";

    pub fn severity_label(&self) -> &'static str {
        self.severity.label()
    }

    pub fn confidence_label(&self) -> &'static str {
        self.confidence.label()
    }

    pub fn populate_recommendation(&mut self) {
        if self.recommendation.trim().is_empty() {
            self.recommendation = Self::recommendation_for_rule_id(&self.rule_id);
        }
    }

    pub fn recommendation_for_rule_id(rule_id: &str) -> String {
        crate::rules::lookup_rule_metadata(rule_id)
            .and_then(|metadata| metadata.recommendation)
            .unwrap_or(Self::GENERIC_RECOMMENDATION)
            .to_string()
    }

    pub fn recommendation_or_default(&self) -> &str {
        if self.recommendation.trim().is_empty() {
            Self::GENERIC_RECOMMENDATION
        } else {
            self.recommendation.as_str()
        }
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

    pub fn lowercase_label(&self) -> &'static str {
        match self {
            Severity::Info => "info",
            Severity::Low => "low",
            Severity::Medium => "medium",
            Severity::High => "high",
            Severity::Critical => "critical",
        }
    }

    pub fn is_at_least(&self, threshold: &Severity) -> bool {
        self >= threshold
    }

    pub fn from_lowercase_label(value: &str) -> Option<Self> {
        match value {
            "info" => Some(Severity::Info),
            "low" => Some(Severity::Low),
            "medium" => Some(Severity::Medium),
            "high" => Some(Severity::High),
            "critical" => Some(Severity::Critical),
            _ => None,
        }
    }
}

impl Confidence {
    pub fn label(&self) -> &'static str {
        match self {
            Confidence::Low => "LOW",
            Confidence::Medium => "MEDIUM",
            Confidence::High => "HIGH",
        }
    }

    pub fn lowercase_label(&self) -> &'static str {
        match self {
            Confidence::Low => "low",
            Confidence::Medium => "medium",
            Confidence::High => "high",
        }
    }
}
