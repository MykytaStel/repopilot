use crate::findings::filter::recompute_summary_metrics;
use crate::findings::quality::summarize_signal_quality;
use crate::findings::types::Finding;
use crate::scan::types::{HiddenSuggestionSummary, ScanSummary};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingVisibilityProfile {
    Default,
    Strict,
}

impl FindingVisibilityProfile {
    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Strict => "strict",
        }
    }
}

/// Product-level intent for a finding.
///
/// Intent is deliberately separate from severity. A high-severity large file can
/// still be a maintainability signal, while a medium/high-confidence runtime
/// finding can be important when it affects production behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingIntent {
    /// Confirmed or high-confidence security issue.
    SecurityRisk,
    /// Runtime/process failure risk that can affect production behavior.
    RuntimeRisk,
    /// High-impact architecture, coupling, framework, or release-blocking risk.
    ActionableRisk,
    /// Maintainability/design signal that is useful in strict/deep audit mode.
    Maintainability,
    /// Test coverage or test-layout signal.
    TestingGap,
    /// Informational or low-signal finding.
    Informational,
}

impl FindingIntent {
    pub fn label(self) -> &'static str {
        match self {
            Self::SecurityRisk => "security-risk",
            Self::RuntimeRisk => "runtime-risk",
            Self::ActionableRisk => "actionable-risk",
            Self::Maintainability => "maintainability",
            Self::TestingGap => "testing-gap",
            Self::Informational => "informational",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FindingVisibilityDecision {
    pub intent: FindingIntent,
    pub visible_by_default: bool,
    pub reason: &'static str,
}

impl FindingVisibilityDecision {
    pub fn hidden(intent: FindingIntent, reason: &'static str) -> Self {
        Self {
            intent,
            visible_by_default: false,
            reason,
        }
    }

    pub fn visible(intent: FindingIntent, reason: &'static str) -> Self {
        Self {
            intent,
            visible_by_default: true,
            reason,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct HiddenSuggestionKey {
    intent: String,
    rule_id: String,
    category: String,
    reason: String,
}

/// Apply the selected visibility profile to a scan summary.
///
/// Strict profile preserves all raw findings. Default profile keeps only
/// findings that should be actionable in a normal development/release loop and
/// stores a structured breakdown for hidden suggestions.
pub fn apply_visibility_profile(summary: &mut ScanSummary, profile: FindingVisibilityProfile) {
    summary.visibility_profile = Some(profile.label().to_string());
    summary.metrics.raw_findings_count = summary.artifacts.findings.len();
    summary.artifacts.raw_signal_quality = summarize_signal_quality(&summary.artifacts.findings);
    summary.metrics.visible_findings_count = summary.artifacts.findings.len();
    summary.metrics.hidden_suggestions_count = 0;
    summary.artifacts.hidden_suggestions.clear();

    if profile == FindingVisibilityProfile::Strict {
        recompute_summary_metrics(summary);
        return;
    }

    let hidden_suggestions = build_hidden_suggestion_summaries(&summary.artifacts.findings);
    let original_count = summary.artifacts.findings.len();

    summary.artifacts.findings.retain(is_visible_by_default);
    summary.metrics.visible_findings_count = summary.artifacts.findings.len();
    summary.metrics.hidden_suggestions_count =
        original_count.saturating_sub(summary.artifacts.findings.len());
    summary.artifacts.hidden_suggestions = hidden_suggestions;
    recompute_summary_metrics(summary);
}

pub fn is_visible_by_default(finding: &Finding) -> bool {
    classify_visibility(finding).visible_by_default
}

/// Build a stable, report-friendly breakdown for findings hidden by default.
///
/// This powers console/Markdown output and also appears in JSON reports through
/// ScanSummary serialization.
pub fn build_hidden_suggestion_summaries(findings: &[Finding]) -> Vec<HiddenSuggestionSummary> {
    let mut counts: BTreeMap<HiddenSuggestionKey, usize> = BTreeMap::new();

    for finding in findings {
        let decision = classify_visibility(finding);
        if decision.visible_by_default {
            continue;
        }

        let key = HiddenSuggestionKey {
            intent: decision.intent.label().to_string(),
            rule_id: finding.rule_id.clone(),
            category: finding.category.label().to_string(),
            reason: decision.reason.to_string(),
        };

        *counts.entry(key).or_insert(0) += 1;
    }

    let mut summaries = counts
        .into_iter()
        .map(|(key, count)| HiddenSuggestionSummary {
            intent: key.intent,
            rule_id: key.rule_id,
            category: key.category,
            reason: key.reason,
            count,
        })
        .collect::<Vec<_>>();

    summaries.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.intent.cmp(&right.intent))
            .then_with(|| left.rule_id.cmp(&right.rule_id))
            .then_with(|| left.reason.cmp(&right.reason))
    });

    summaries
}

/// Classify a finding into an intent and default visibility decision.
///
/// This function is the main policy boundary. Audit rules should describe
/// evidence and severity; this layer decides how noisy the default report is.
mod policy;

pub use policy::classify_visibility;

#[cfg(test)]
mod tests;
