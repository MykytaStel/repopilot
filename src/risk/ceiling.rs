//! risk-v4 severity ceiling. Context signals (baseline status, review diff,
//! graph, clusters) reorder findings within the priority band their severity
//! allows, but never lift a finding above it: a medium-severity structural
//! finding cannot become P0 just because it is new and in the diff.

use super::model::{RiskAssessment, RiskPriority, priority_for_score, signal};
use crate::findings::types::Severity;

/// Signal recorded when the ceiling lowers the score-derived priority.
pub const PRIORITY_CEILING_SIGNAL: &str = "severity.priority-ceiling";

/// Highest priority a finding of `severity` may receive.
pub fn severity_priority_ceiling(severity: Severity) -> RiskPriority {
    match severity {
        Severity::Critical | Severity::High => RiskPriority::P0,
        Severity::Medium => RiskPriority::P1,
        Severity::Low => RiskPriority::P2,
        Severity::Info => RiskPriority::P3,
    }
}

/// Priority for `score`, capped by the severity ceiling.
pub fn priority_for(score: u8, severity: Severity) -> RiskPriority {
    let by_score = priority_for_score(score);
    let ceiling = severity_priority_ceiling(severity);
    if by_score.rank() < ceiling.rank() {
        ceiling
    } else {
        by_score
    }
}

/// Recompute `risk.priority` from its score and keep the ceiling signal in
/// sync, so reports explain why a high score maps to a lower priority.
pub(super) fn apply_priority(risk: &mut RiskAssessment, severity: Severity) {
    let by_score = priority_for_score(risk.score);
    risk.priority = priority_for(risk.score, severity);
    let capped = risk.priority != by_score;
    risk.signals
        .retain(|existing| existing.id != PRIORITY_CEILING_SIGNAL);
    if capped {
        risk.signals.push(signal(
            PRIORITY_CEILING_SIGNAL,
            "priority ceiling",
            0,
            &format!(
                "{} severity caps priority at {}",
                severity.label().to_lowercase(),
                risk.priority.label()
            ),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::risk::FORMULA_VERSION;

    fn assessment(score: u8) -> RiskAssessment {
        RiskAssessment {
            score,
            priority: priority_for_score(score),
            signals: Vec::new(),
            formula_version: FORMULA_VERSION.to_string(),
        }
    }

    #[test]
    fn severity_caps_context_driven_priority() {
        assert_eq!(priority_for(94, Severity::Medium), RiskPriority::P1);
        assert_eq!(priority_for(94, Severity::Low), RiskPriority::P2);
        assert_eq!(priority_for(94, Severity::Info), RiskPriority::P3);
        assert_eq!(priority_for(94, Severity::High), RiskPriority::P0);
        assert_eq!(priority_for(94, Severity::Critical), RiskPriority::P0);
    }

    #[test]
    fn ceiling_never_raises_a_low_score() {
        assert_eq!(priority_for(45, Severity::Critical), RiskPriority::P2);
        assert_eq!(priority_for(10, Severity::Medium), RiskPriority::P3);
    }

    #[test]
    fn capped_priority_records_one_explaining_signal() {
        let mut risk = assessment(94);
        apply_priority(&mut risk, Severity::Medium);
        apply_priority(&mut risk, Severity::Medium);

        assert_eq!(risk.priority, RiskPriority::P1);
        let ceiling = risk
            .signals
            .iter()
            .filter(|signal| signal.id == PRIORITY_CEILING_SIGNAL)
            .collect::<Vec<_>>();
        assert_eq!(ceiling.len(), 1);
        assert_eq!(ceiling[0].weight, 0);
        assert_eq!(ceiling[0].reason, "medium severity caps priority at P1");
    }

    #[test]
    fn ceiling_signal_is_dropped_when_the_score_falls_back_under_it() {
        let mut risk = assessment(94);
        apply_priority(&mut risk, Severity::Medium);
        risk.score = 80;
        apply_priority(&mut risk, Severity::Medium);

        assert_eq!(risk.priority, RiskPriority::P1);
        assert!(
            !risk
                .signals
                .iter()
                .any(|signal| signal.id == PRIORITY_CEILING_SIGNAL)
        );
    }
}
