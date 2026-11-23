use super::*;

pub(super) fn check_applicability(
    rule: &RuleApplicability,
    context: &RuleMatchContext<'_>,
    trace: &mut TraceRecorder<'_>,
) -> Option<RuleDecision> {
    if let Some(decision) = check_dimension(
        trace,
        "language",
        !rule.languages.is_empty(),
        context
            .languages
            .iter()
            .any(|language| rule.languages.contains(*language)),
        || {
            vec![
                format!("allowed={}", sorted_values(&rule.languages)),
                format!("actual={}", joined_ids(context.languages)),
            ]
        },
        SuppressReason::LanguageNotMatched,
        context.base_severity,
    ) {
        return Some(decision);
    }
    if let Some(decision) = check_dimension(
        trace,
        "language-support",
        rule.minimum_support.is_some(),
        rule.minimum_support
            .is_some_and(|minimum| language_support_satisfies(rule, context.languages, minimum)),
        || {
            vec![
                format!(
                    "minimum={}",
                    rule.minimum_support.map(support_level_id).unwrap_or("none")
                ),
                format!("languages={}", joined_ids(context.languages)),
            ]
        },
        SuppressReason::LanguageSupportTooLow,
        context.base_severity,
    ) {
        return Some(decision);
    }
    if let Some(decision) = check_dimension(
        trace,
        "framework",
        !rule.frameworks.is_empty(),
        context
            .frameworks
            .iter()
            .any(|framework| rule.frameworks.contains(*framework)),
        || {
            vec![
                format!("allowed={}", sorted_values(&rule.frameworks)),
                format!("actual={}", joined_ids(context.frameworks)),
            ]
        },
        SuppressReason::FrameworkNotMatched,
        context.base_severity,
    ) {
        return Some(decision);
    }
    if let Some(decision) = check_dimension(
        trace,
        "runtime",
        !rule.runtimes.is_empty(),
        context
            .runtimes
            .iter()
            .any(|runtime| rule.runtimes.contains(*runtime)),
        || {
            vec![
                format!("allowed={}", sorted_values(&rule.runtimes)),
                format!("actual={}", joined_ids(context.runtimes)),
            ]
        },
        SuppressReason::RuntimeNotMatched,
        context.base_severity,
    ) {
        return Some(decision);
    }
    if let Some(decision) = check_dimension(
        trace,
        "paradigm",
        !rule.paradigms.is_empty(),
        context
            .paradigms
            .iter()
            .any(|paradigm| rule.paradigms.contains(*paradigm)),
        || {
            vec![
                format!("allowed={}", sorted_values(&rule.paradigms)),
                format!("actual={}", joined_ids(context.paradigms)),
            ]
        },
        SuppressReason::ParadigmNotMatched,
        context.base_severity,
    ) {
        return Some(decision);
    }
    check_role_applicability(rule, context, trace)
}

fn check_dimension<F>(
    trace: &mut TraceRecorder<'_>,
    label: &'static str,
    enabled: bool,
    passed: bool,
    criteria: F,
    reason: SuppressReason,
    base_severity: Severity,
) -> Option<RuleDecision>
where
    F: FnOnce() -> Vec<String>,
{
    enabled.then(|| record_applicability(trace, label, criteria, passed, reason, base_severity))?
}

fn check_role_applicability(
    rule: &RuleApplicability,
    context: &RuleMatchContext<'_>,
    trace: &mut TraceRecorder<'_>,
) -> Option<RuleDecision> {
    if rule.suppress_low_signal
        && let Some(decision) = record_applicability(
            trace,
            "low-signal-path",
            || vec![format!("is_low_signal={}", context.is_low_signal)],
            !context.is_low_signal,
            SuppressReason::LowSignalPath,
            context.base_severity,
        )
    {
        return Some(decision);
    }
    if rule.suppress_config {
        let is_config = context.roles.contains(&"config");
        if let Some(decision) = record_applicability(
            trace,
            "config-role",
            || vec![format!("roles={}", joined_ids(context.roles))],
            !is_config,
            SuppressReason::ConfigFile,
            context.base_severity,
        ) {
            return Some(decision);
        }
    }
    if rule.suppress_generated {
        let is_generated = context.roles.contains(&"generated");
        if let Some(decision) = record_applicability(
            trace,
            "generated-role",
            || vec![format!("roles={}", joined_ids(context.roles))],
            !is_generated,
            SuppressReason::GeneratedFile,
            context.base_severity,
        ) {
            return Some(decision);
        }
    }
    None
}

fn record_applicability<F>(
    trace: &mut TraceRecorder<'_>,
    label: &'static str,
    criteria: F,
    passed: bool,
    failure: SuppressReason,
    severity: Severity,
) -> Option<RuleDecision>
where
    F: FnOnce() -> Vec<String>,
{
    let decision = if passed {
        None
    } else {
        Some(RuleDecision::suppress(failure))
    };
    let action = decision.as_ref().map(|_| RuleDecisionAction::Suppress);
    let severity_after = decision.as_ref().map_or(severity, |value| value.severity);

    trace.push(|| DecisionTraceStep {
        stage: DecisionTraceStage::Applicability,
        status: if passed {
            DecisionTraceStatus::Passed
        } else {
            DecisionTraceStatus::Failed
        },
        label: label.to_string(),
        criteria: criteria(),
        action,
        severity_before: severity,
        severity_after,
        reason: if passed {
            format!("{label} applicability passed")
        } else {
            failure.to_string()
        },
        override_index: None,
    });
    decision
}
