use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::findings::provenance::FindingProvenance;
pub use crate::findings::severity::Severity;

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
    #[serde(default)]
    pub provenance: FindingProvenance,
    #[serde(default)]
    pub risk: crate::risk::RiskAssessment,
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
        self.populate_rule_metadata();
    }

    pub fn populate_rule_metadata(&mut self) {
        let Some(metadata) = crate::rules::lookup_rule_metadata(&self.rule_id) else {
            if self.recommendation.trim().is_empty() {
                self.recommendation = Self::GENERIC_RECOMMENDATION.to_string();
            }
            return;
        };

        if self.title.trim().is_empty() {
            self.title = metadata.title.to_string();
        }
        if self.description.trim().is_empty() {
            self.description = metadata.description.to_string();
        }
        if self.recommendation.trim().is_empty() {
            self.recommendation = metadata
                .recommendation
                .unwrap_or(Self::GENERIC_RECOMMENDATION)
                .to_string();
        }
        if self.docs_url.as_deref().is_none_or(str::is_empty) {
            self.docs_url = metadata.docs_url.map(str::to_string);
        }
        if self.provenance.has_default_metadata() {
            let analysis_scope = self.provenance.analysis_scope;
            let knowledge_decision = self.provenance.knowledge_decision.take();
            self.provenance = FindingProvenance {
                detector: metadata.rule_id.to_string(),
                signal_source: metadata.signal_source,
                rule_lifecycle: metadata.lifecycle,
                analysis_scope,
                knowledge_decision,
            };
        }

        // The rule registry owns severity and confidence. Context (audit tiers,
        // knowledge-pack overrides) may lower severity or raise it up to the
        // declared ceiling, never above it.
        //
        // `Severity::Info` doubles as both the lowest real severity and the
        // "audit never set this" sentinel (it's `Severity::default()`), so a
        // bare `severity == Info` check can't tell a genuine Info decision
        // apart from an untouched field. `knowledge_decision` is only stamped
        // once a finding has passed through the decision engine
        // (`apply_file_decision` / `apply_project_decisions` / a direct
        // `record_decision_provenance` call) with a real severity already
        // assigned by the audit, so its presence is what distinguishes
        // "deliberately Info" from "never set" — mirroring the
        // `Confidence::Medium`-as-sentinel caveat documented in
        // `graph_queries/mod.rs`.
        let is_default_severity =
            self.severity == Severity::Info && self.provenance.knowledge_decision.is_none();
        if is_default_severity {
            self.severity = metadata.default_severity;
        } else {
            self.severity = self.severity.min(metadata.severity_ceiling());
        }

        // Confidence is fixed to the registry default unless the rule declares
        // contextual confidence, in which case the audit's per-finding value is
        // kept but capped at the ceiling.
        let is_default_confidence = self.confidence == Confidence::Medium;
        if !metadata.contextual_confidence || is_default_confidence {
            self.confidence = metadata.default_confidence;
        } else {
            self.confidence = self.confidence.min(metadata.confidence_ceiling());
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

#[cfg(test)]
mod tests;
