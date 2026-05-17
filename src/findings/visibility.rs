use crate::findings::types::{Confidence, Finding, FindingCategory, Severity};
use crate::risk::RiskPriority;
use crate::scan::types::ScanSummary;

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

pub fn apply_visibility_profile(summary: &mut ScanSummary, profile: FindingVisibilityProfile) {
    summary.visibility_profile = Some(profile.label().to_string());
    summary.visible_findings_count = summary.findings.len();
    summary.hidden_suggestions_count = 0;

    if profile == FindingVisibilityProfile::Strict {
        return;
    }

    let original_count = summary.findings.len();
    summary.findings.retain(is_visible_by_default);
    summary.visible_findings_count = summary.findings.len();
    summary.hidden_suggestions_count = original_count.saturating_sub(summary.findings.len());
    summary.health_score =
        ScanSummary::compute_health_score(&summary.findings, summary.lines_of_code);
}

pub fn is_visible_by_default(finding: &Finding) -> bool {
    if is_validated_secret_leak(finding) {
        return true;
    }

    if is_hidden_suggestion_by_default(finding) {
        return false;
    }

    if matches!(finding.risk.priority, RiskPriority::P0 | RiskPriority::P1) {
        return true;
    }

    if finding.category == FindingCategory::Security && finding.severity >= Severity::High {
        return true;
    }

    finding.severity >= Severity::High
        && finding.confidence == Confidence::High
        && finding.category != FindingCategory::Testing
}

fn is_hidden_suggestion_by_default(finding: &Finding) -> bool {
    if finding.severity <= Severity::Low {
        return true;
    }

    matches!(
        finding.rule_id.as_str(),
        "testing.source-without-test"
            | "code-marker.todo"
            | "architecture.deep-nesting"
            | "architecture.too-many-modules"
            | "architecture.large-file"
            | "code-quality.long-function"
            | "code-quality.complex-file"
    )
}

fn is_validated_secret_leak(finding: &Finding) -> bool {
    matches!(
        finding.rule_id.as_str(),
        "security.secret-candidate" | "security.private-key-candidate"
    ) && finding.severity >= Severity::High
}
