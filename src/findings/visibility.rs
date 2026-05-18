use crate::findings::types::{Confidence, Finding, FindingCategory, Severity};
use crate::risk::RiskPriority;
use crate::scan::types::{HiddenSuggestionSummary, ScanSummary};
use std::collections::BTreeMap;
use std::path::Path;

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
    summary.visible_findings_count = summary.findings.len();
    summary.hidden_suggestions_count = 0;
    summary.hidden_suggestions.clear();

    if profile == FindingVisibilityProfile::Strict {
        return;
    }

    let hidden_suggestions = build_hidden_suggestion_summaries(&summary.findings);
    let original_count = summary.findings.len();

    summary.findings.retain(is_visible_by_default);
    summary.visible_findings_count = summary.findings.len();
    summary.hidden_suggestions_count = original_count.saturating_sub(summary.findings.len());
    summary.hidden_suggestions = hidden_suggestions;
    summary.health_score =
        ScanSummary::compute_health_score(&summary.findings, summary.non_empty_lines);
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
pub fn classify_visibility(finding: &Finding) -> FindingVisibilityDecision {
    let intent = classify_intent(finding);

    if is_validated_secret_leak(finding) {
        return FindingVisibilityDecision::visible(
            FindingIntent::SecurityRisk,
            "validated secret or private key candidate",
        );
    }

    if finding.severity <= Severity::Low {
        return FindingVisibilityDecision::hidden(
            intent,
            "low-severity findings are strict-mode suggestions by default",
        );
    }

    match intent {
        FindingIntent::SecurityRisk => security_visibility(finding),
        FindingIntent::RuntimeRisk => runtime_visibility(finding),
        FindingIntent::ActionableRisk => actionable_visibility(finding),
        FindingIntent::Maintainability => FindingVisibilityDecision::hidden(
            intent,
            "maintainability signals are hidden in the default profile",
        ),
        FindingIntent::TestingGap => FindingVisibilityDecision::hidden(
            intent,
            "testing gaps are hidden in the default profile",
        ),
        FindingIntent::Informational => FindingVisibilityDecision::hidden(
            intent,
            "informational findings are hidden in the default profile",
        ),
    }
}

fn security_visibility(finding: &Finding) -> FindingVisibilityDecision {
    if finding.severity >= Severity::High && finding.confidence != Confidence::Low {
        return FindingVisibilityDecision::visible(
            FindingIntent::SecurityRisk,
            "high-confidence security risk",
        );
    }

    FindingVisibilityDecision::hidden(
        FindingIntent::SecurityRisk,
        "security signal is below the default confidence/severity threshold",
    )
}

fn runtime_visibility(finding: &Finding) -> FindingVisibilityDecision {
    if is_script_boundary_runtime_exit(finding) {
        return FindingVisibilityDecision::hidden(
            FindingIntent::RuntimeRisk,
            "process exit in script/tooling boundary is a strict-mode suggestion",
        );
    }

    if is_high_priority(finding.risk.priority) {
        return FindingVisibilityDecision::visible(
            FindingIntent::RuntimeRisk,
            "high-priority runtime risk",
        );
    }

    if finding.severity >= Severity::High && finding.confidence != Confidence::Low {
        return FindingVisibilityDecision::visible(
            FindingIntent::RuntimeRisk,
            "high-severity runtime risk",
        );
    }

    FindingVisibilityDecision::hidden(
        FindingIntent::RuntimeRisk,
        "runtime signal is not actionable enough for the default profile",
    )
}

fn actionable_visibility(finding: &Finding) -> FindingVisibilityDecision {
    if is_high_priority(finding.risk.priority) {
        return FindingVisibilityDecision::visible(
            FindingIntent::ActionableRisk,
            "high-priority actionable risk",
        );
    }

    if finding.severity >= Severity::High && finding.confidence == Confidence::High {
        return FindingVisibilityDecision::visible(
            FindingIntent::ActionableRisk,
            "high-severity high-confidence actionable risk",
        );
    }

    FindingVisibilityDecision::hidden(
        FindingIntent::ActionableRisk,
        "actionable signal is below the default visibility threshold",
    )
}

fn classify_intent(finding: &Finding) -> FindingIntent {
    if finding.category == FindingCategory::Testing {
        return FindingIntent::TestingGap;
    }

    if finding.category == FindingCategory::Security {
        return FindingIntent::SecurityRisk;
    }

    if is_runtime_rule(&finding.rule_id) {
        return FindingIntent::RuntimeRisk;
    }

    if is_maintainability_rule(&finding.rule_id) {
        return FindingIntent::Maintainability;
    }

    if finding.severity >= Severity::High {
        return FindingIntent::ActionableRisk;
    }

    FindingIntent::Informational
}

fn is_runtime_rule(rule_id: &str) -> bool {
    matches!(
        rule_id,
        "language.rust.panic-risk"
            | "language.javascript.runtime-exit-risk"
            | "language.go.panic-exit-risk"
            | "language.python.exception-risk"
            | "language.jvm.exception-risk"
            | "language.csharp.exception-risk"
    )
}

fn is_maintainability_rule(rule_id: &str) -> bool {
    matches!(
        rule_id,
        "testing.source-without-test"
            | "code-marker.todo"
            | "architecture.deep-nesting"
            | "architecture.too-many-modules"
            | "architecture.large-file"
            | "architecture.barrel-file-risk"
            | "architecture.deep-relative-import"
            | "code-quality.long-function"
            | "code-quality.complex-file"
            | "code-quality.cyclomatic-complexity"
    )
}

fn is_validated_secret_leak(finding: &Finding) -> bool {
    matches!(
        finding.rule_id.as_str(),
        "security.secret-candidate" | "security.private-key-candidate"
    ) && finding.severity >= Severity::High
        && finding.confidence != Confidence::Low
}

fn is_high_priority(priority: RiskPriority) -> bool {
    matches!(priority, RiskPriority::P0 | RiskPriority::P1)
}

fn is_script_boundary_runtime_exit(finding: &Finding) -> bool {
    if finding.rule_id != "language.javascript.runtime-exit-risk" {
        return false;
    }

    let Some(evidence) = finding.evidence.first() else {
        return false;
    };

    is_script_or_tooling_path(&evidence.path)
}

fn is_script_or_tooling_path(path: &Path) -> bool {
    let path_text = path.to_string_lossy().replace("\"", "/").to_lowercase();

    if path_text.contains("/src/") || path_text.starts_with("src/") {
        return false;
    }

    path_text.contains("/scripts/")
        || path_text.starts_with("scripts/")
        || path_text.contains("/tools/")
        || path_text.starts_with("tools/")
        || path_text.contains("/bin/")
        || path_text.starts_with("bin/")
        || path_text.contains("/ci/")
        || path_text.starts_with("ci/")
        || path_text.contains("/.github/")
        || path_text.starts_with(".github/")
        || path_text.contains("guard")
        || path_text.contains("check")
        || path_text.contains("lint")
        || path_text.contains("verify")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::findings::types::{Evidence, FindingCategory};
    use std::path::PathBuf;

    fn finding(rule_id: &str, category: FindingCategory, severity: Severity) -> Finding {
        Finding {
            rule_id: rule_id.to_string(),
            category,
            severity,
            confidence: Confidence::High,
            ..Default::default()
        }
    }

    fn finding_with_path(
        rule_id: &str,
        category: FindingCategory,
        severity: Severity,
        path: &str,
    ) -> Finding {
        let mut finding = finding(rule_id, category, severity);
        finding.evidence = vec![Evidence {
            path: PathBuf::from(path),
            line_start: 1,
            line_end: None,
            snippet: "process.exit(1);".to_string(),
        }];
        finding
    }

    #[test]
    fn default_profile_hides_testing_gaps_by_intent() {
        let finding = finding(
            "testing.source-without-test",
            FindingCategory::Testing,
            Severity::Low,
        );

        let decision = classify_visibility(&finding);

        assert_eq!(decision.intent, FindingIntent::TestingGap);
        assert!(!decision.visible_by_default);
    }

    #[test]
    fn default_profile_hides_high_maintainability_signals() {
        let finding = finding(
            "architecture.large-file",
            FindingCategory::Architecture,
            Severity::High,
        );

        let decision = classify_visibility(&finding);

        assert_eq!(decision.intent, FindingIntent::Maintainability);
        assert!(!decision.visible_by_default);
    }

    #[test]
    fn default_profile_keeps_validated_secret_candidates() {
        let finding = finding(
            "security.secret-candidate",
            FindingCategory::Security,
            Severity::High,
        );

        let decision = classify_visibility(&finding);

        assert_eq!(decision.intent, FindingIntent::SecurityRisk);
        assert!(decision.visible_by_default);
    }

    #[test]
    fn default_profile_hides_script_process_exit() {
        let finding = finding_with_path(
            "language.javascript.runtime-exit-risk",
            FindingCategory::CodeQuality,
            Severity::High,
            "scripts/verify-release.mjs",
        );

        let decision = classify_visibility(&finding);

        assert_eq!(decision.intent, FindingIntent::RuntimeRisk);
        assert!(!decision.visible_by_default);
    }

    #[test]
    fn default_profile_keeps_source_process_exit() {
        let finding = finding_with_path(
            "language.javascript.runtime-exit-risk",
            FindingCategory::CodeQuality,
            Severity::High,
            "src/runtime/server.ts",
        );

        let decision = classify_visibility(&finding);

        assert_eq!(decision.intent, FindingIntent::RuntimeRisk);
        assert!(decision.visible_by_default);
    }

    #[test]
    fn hidden_suggestion_summary_groups_by_intent_rule_and_reason() {
        let findings = vec![
            finding(
                "architecture.large-file",
                FindingCategory::Architecture,
                Severity::High,
            ),
            finding(
                "architecture.large-file",
                FindingCategory::Architecture,
                Severity::High,
            ),
            finding(
                "testing.source-without-test",
                FindingCategory::Testing,
                Severity::Low,
            ),
        ];

        let summaries = build_hidden_suggestion_summaries(&findings);

        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0].rule_id, "architecture.large-file");
        assert_eq!(summaries[0].intent, "maintainability");
        assert_eq!(summaries[0].count, 2);
        assert_eq!(summaries[1].rule_id, "testing.source-without-test");
        assert_eq!(summaries[1].intent, "testing-gap");
        assert_eq!(summaries[1].count, 1);
    }

    #[test]
    fn apply_default_profile_recomputes_visible_counts_and_hidden_breakdown() {
        let visible = finding(
            "security.secret-candidate",
            FindingCategory::Security,
            Severity::High,
        );
        let hidden = finding(
            "architecture.large-file",
            FindingCategory::Architecture,
            Severity::High,
        );

        let mut summary = ScanSummary {
            hidden_suggestions: Vec::new(),
            non_empty_lines: 1_000,
            findings: vec![visible, hidden],
            ..Default::default()
        };

        apply_visibility_profile(&mut summary, FindingVisibilityProfile::Default);

        assert_eq!(summary.visibility_profile.as_deref(), Some("default"));
        assert_eq!(summary.visible_findings_count, 1);
        assert_eq!(summary.hidden_suggestions_count, 1);
        assert_eq!(summary.hidden_suggestions.len(), 1);
        assert_eq!(summary.hidden_suggestions[0].intent, "maintainability");
        assert_eq!(
            summary.hidden_suggestions[0].rule_id,
            "architecture.large-file"
        );
        assert_eq!(summary.findings.len(), 1);
        assert_eq!(summary.findings[0].rule_id, "security.secret-candidate");
    }

    #[test]
    fn strict_profile_keeps_all_findings_and_clears_hidden_breakdown() {
        let mut summary = ScanSummary {
            non_empty_lines: 1_000,
            hidden_suggestions: vec![HiddenSuggestionSummary {
                intent: "maintainability".to_string(),
                rule_id: "architecture.large-file".to_string(),
                category: "architecture".to_string(),
                reason: "example".to_string(),
                count: 1,
            }],
            findings: vec![
                finding(
                    "security.secret-candidate",
                    FindingCategory::Security,
                    Severity::High,
                ),
                finding(
                    "architecture.large-file",
                    FindingCategory::Architecture,
                    Severity::High,
                ),
            ],
            ..Default::default()
        };

        apply_visibility_profile(&mut summary, FindingVisibilityProfile::Strict);

        assert_eq!(summary.visibility_profile.as_deref(), Some("strict"));
        assert_eq!(summary.visible_findings_count, 2);
        assert_eq!(summary.hidden_suggestions_count, 0);
        assert!(summary.hidden_suggestions.is_empty());
        assert_eq!(summary.findings.len(), 2);
    }
}
