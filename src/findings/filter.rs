use crate::findings::quality::summarize_signal_quality;
use crate::findings::types::{Confidence, Finding, Severity};
use crate::risk::RiskPriority;
use crate::scan::types::ScanSummary;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FindingFilter {
    pub min_severity: Option<Severity>,
    pub min_confidence: Option<Confidence>,
    pub min_priority: Option<RiskPriority>,
    pub rule_ids: Vec<String>,
}

impl FindingFilter {
    pub fn is_empty(&self) -> bool {
        self.min_severity.is_none()
            && self.min_confidence.is_none()
            && self.min_priority.is_none()
            && self.rule_ids.is_empty()
    }

    pub fn matches(&self, finding: &Finding) -> bool {
        if let Some(min) = self.min_severity
            && !finding.severity.is_at_least(&min)
        {
            return false;
        }

        if let Some(min) = self.min_confidence
            && finding.confidence < min
        {
            return false;
        }

        if let Some(min) = self.min_priority
            && !finding.risk.priority.is_at_least(min)
        {
            return false;
        }

        if !self.rule_ids.is_empty()
            && !self
                .rule_ids
                .iter()
                .any(|rule_id| rule_id == &finding.rule_id)
        {
            return false;
        }

        true
    }

    pub fn apply_to_summary(&self, summary: &mut ScanSummary) {
        if !self.is_empty() {
            summary
                .artifacts
                .findings
                .retain(|finding| self.matches(finding));
        }
        recompute_summary_metrics(summary);
    }
}

pub fn recompute_summary_metrics(summary: &mut ScanSummary) {
    summary.metrics.visible_findings_count = summary.artifacts.findings.len();
    summary.metrics.health_score = ScanSummary::compute_health_score(
        &summary.artifacts.findings,
        summary.metrics.non_empty_lines,
    );
    let visible_signal_quality = summarize_signal_quality(&summary.artifacts.findings);
    if summary.metrics.raw_findings_count == 0
        && summary.metrics.hidden_suggestions_count == 0
        && !summary.artifacts.findings.is_empty()
    {
        summary.metrics.raw_findings_count = summary.artifacts.findings.len();
        summary.artifacts.raw_signal_quality = visible_signal_quality.clone();
    }
    summary.artifacts.visible_signal_quality = visible_signal_quality.clone();
    summary.artifacts.signal_quality = visible_signal_quality;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::risk::RiskAssessment;
    use crate::scan::types::{ScanArtifacts, ScanMetadata, ScanMetrics};
    use std::path::PathBuf;

    fn finding(
        rule_id: &str,
        severity: Severity,
        confidence: Confidence,
        priority: RiskPriority,
    ) -> Finding {
        Finding {
            rule_id: rule_id.to_string(),
            severity,
            confidence,
            provenance: Default::default(),
            risk: RiskAssessment {
                priority,
                ..RiskAssessment::default()
            },
            ..Finding::default()
        }
    }

    #[test]
    fn severity_threshold_matches_at_or_above_minimum() {
        let filter = FindingFilter {
            min_severity: Some(Severity::High),
            ..FindingFilter::default()
        };

        assert!(filter.matches(&finding(
            "rule.high",
            Severity::High,
            Confidence::Medium,
            RiskPriority::P3
        )));
        assert!(!filter.matches(&finding(
            "rule.medium",
            Severity::Medium,
            Confidence::Medium,
            RiskPriority::P3
        )));
    }

    #[test]
    fn confidence_threshold_matches_at_or_above_minimum() {
        let filter = FindingFilter {
            min_confidence: Some(Confidence::Medium),
            ..FindingFilter::default()
        };

        assert!(filter.matches(&finding(
            "rule.medium",
            Severity::Low,
            Confidence::Medium,
            RiskPriority::P3
        )));
        assert!(!filter.matches(&finding(
            "rule.low",
            Severity::Low,
            Confidence::Low,
            RiskPriority::P3
        )));
    }

    #[test]
    fn priority_threshold_matches_at_or_above_minimum() {
        let filter = FindingFilter {
            min_priority: Some(RiskPriority::P1),
            ..FindingFilter::default()
        };

        assert!(filter.matches(&finding(
            "rule.p1",
            Severity::Low,
            Confidence::Medium,
            RiskPriority::P1
        )));
        assert!(!filter.matches(&finding(
            "rule.p2",
            Severity::Low,
            Confidence::Medium,
            RiskPriority::P2
        )));
    }

    #[test]
    fn repeated_rule_filter_matches_any_rule_id() {
        let filter = FindingFilter {
            rule_ids: vec!["rule.one".to_string(), "rule.two".to_string()],
            ..FindingFilter::default()
        };

        assert!(filter.matches(&finding(
            "rule.two",
            Severity::Low,
            Confidence::Medium,
            RiskPriority::P3
        )));
        assert!(!filter.matches(&finding(
            "rule.three",
            Severity::Low,
            Confidence::Medium,
            RiskPriority::P3
        )));
    }

    #[test]
    fn combined_filter_requires_every_selected_threshold() {
        let filter = FindingFilter {
            min_severity: Some(Severity::Medium),
            min_confidence: Some(Confidence::High),
            min_priority: Some(RiskPriority::P2),
            rule_ids: vec!["rule.keep".to_string()],
        };

        assert!(filter.matches(&finding(
            "rule.keep",
            Severity::High,
            Confidence::High,
            RiskPriority::P2
        )));
        assert!(!filter.matches(&finding(
            "rule.keep",
            Severity::High,
            Confidence::Medium,
            RiskPriority::P2
        )));
        assert!(!filter.matches(&finding(
            "rule.drop",
            Severity::High,
            Confidence::High,
            RiskPriority::P2
        )));
    }

    #[test]
    fn apply_to_summary_recomputes_visible_count_and_health_score() {
        let filter = FindingFilter {
            min_severity: Some(Severity::High),
            ..FindingFilter::default()
        };
        let mut summary = ScanSummary {
            metadata: ScanMetadata {
                root_path: PathBuf::from("."),
                ..Default::default()
            },
            metrics: ScanMetrics {
                non_empty_lines: 1_000,
                ..Default::default()
            },
            artifacts: ScanArtifacts {
                findings: vec![
                    finding(
                        "rule.keep",
                        Severity::High,
                        Confidence::Medium,
                        RiskPriority::P3,
                    ),
                    finding(
                        "rule.drop",
                        Severity::Low,
                        Confidence::Medium,
                        RiskPriority::P3,
                    ),
                ],
                ..Default::default()
            },
        };

        filter.apply_to_summary(&mut summary);

        assert_eq!(summary.artifacts.findings.len(), 1);
        assert_eq!(summary.metrics.visible_findings_count, 1);
        assert_eq!(summary.metrics.health_score, 95);
    }
}
