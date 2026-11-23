use super::*;

pub(super) fn apply_overlay(
    context: &RuleMatchContext<'_>,
    decision: RuleDecision,
    trace: &mut TraceRecorder<'_>,
) -> RuleDecision {
    apply_overlay_entries_with_marking(
        context,
        crate::knowledge::overlay::active_overlay(),
        decision,
        trace,
    )
}

pub(super) fn apply_overlay_entries_with_marking(
    context: &RuleMatchContext<'_>,
    rules: &crate::knowledge::overlay::OverlayRules,
    mut decision: RuleDecision,
    trace: &mut TraceRecorder<'_>,
) -> RuleDecision {
    use crate::knowledge::overlay::OverlayTarget;

    for (position, entry) in rules.entries().iter().enumerate() {
        let OverlayTarget::Rule(rule_id) = &entry.target else {
            continue;
        };
        if rule_id != context.rule_id {
            continue;
        }
        if let Some(expires) = entry.expires
            && expires < crate::utc::today()
        {
            continue;
        }
        let path_matches = match (&entry.path_glob, context.path) {
            (None, _) => true,
            (Some(glob), Some(path)) => glob.is_match(path),
            (Some(_), None) => false,
        };
        if !path_matches {
            continue;
        }

        rules.mark_matched(position);

        let before = decision.severity;
        decision = match entry.severity {
            Some(severity) => RuleDecision {
                action: severity_transition_action(before, severity),
                severity,
                reason: entry.reason.clone(),
                risk_signal: decision.risk_signal.clone(),
                via_overlay: true,
            },
            None => RuleDecision::suppress(
                entry
                    .reason
                    .clone()
                    .unwrap_or_else(|| format!("overlay[{}] suppresses this rule", entry.index)),
            )
            .with_via_overlay(true),
        };

        trace.push(|| DecisionTraceStep {
            stage: DecisionTraceStage::Overlay,
            status: DecisionTraceStatus::Applied,
            label: format!("overlay[{}]", entry.index),
            criteria: vec![
                format!("rule={rule_id}"),
                format!(
                    "path={}",
                    entry.path_text.clone().unwrap_or_else(|| "*".to_string())
                ),
            ],
            action: Some(decision.action),
            severity_before: before,
            severity_after: decision.severity,
            reason: decision
                .reason
                .clone()
                .unwrap_or_else(|| format!("overlay[{}] matched", entry.index)),
            override_index: None,
        });

        if decision.is_suppressed() {
            return decision;
        }
    }

    decision
}

#[cfg(test)]
pub(crate) fn apply_overlay_for_test(
    context: &RuleMatchContext<'_>,
    entries: Vec<crate::knowledge::overlay::OverlayEntry>,
) -> RuleDecision {
    let rules = crate::knowledge::overlay::OverlayRules::from_entries_for_test(entries);
    let mut trace = TraceRecorder::disabled();
    let decision = RuleDecision::apply(context.base_severity);
    apply_overlay_entries_with_marking(context, &rules, decision, &mut trace)
}
