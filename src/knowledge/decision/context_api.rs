use super::*;

pub fn decide_for_audit_context(
    rule_id: &str,
    context: &AuditContext,
    base_severity: Severity,
    signal: Option<&str>,
) -> RuleDecision {
    let language_ids = [context.language_id()];
    decide_with_context(
        rule_id,
        &language_ids,
        context,
        false,
        base_severity,
        signal,
        None,
    )
}

pub fn decide_for_audit_context_with_trace(
    rule_id: &str,
    context: &AuditContext,
    base_severity: Severity,
    signal: Option<&str>,
) -> RuleDecisionTrace {
    let language_ids = [context.language_id()];
    decide_with_context_trace(
        rule_id,
        &language_ids,
        context,
        false,
        base_severity,
        signal,
        None,
    )
}

pub fn decide_for_file(
    rule_id: &str,
    file: &FileFacts,
    base_severity: Severity,
    signal: Option<&str>,
) -> RuleDecision {
    let context = classify_file(file);
    let mut language_ids = language_ids_for_file(file, &context);
    dedup_static_ids(&mut language_ids);
    let is_low_signal = is_low_signal_audit_path(&file.path) && !context.is_test;
    let path = file.path.to_str();
    decide_with_context(
        rule_id,
        &language_ids,
        &context,
        is_low_signal,
        base_severity,
        signal,
        path,
    )
}

pub fn decide_for_file_with_trace(
    rule_id: &str,
    file: &FileFacts,
    base_severity: Severity,
    signal: Option<&str>,
) -> RuleDecisionTrace {
    let context = classify_file(file);
    let mut language_ids = language_ids_for_file(file, &context);
    dedup_static_ids(&mut language_ids);
    let is_low_signal = is_low_signal_audit_path(&file.path) && !context.is_test;
    let path = file.path.to_str();
    decide_with_context_trace(
        rule_id,
        &language_ids,
        &context,
        is_low_signal,
        base_severity,
        signal,
        path,
    )
}

fn decide_with_context(
    rule_id: &str,
    language_ids: &[&str],
    context: &AuditContext,
    is_low_signal: bool,
    base_severity: Severity,
    signal: Option<&str>,
    path: Option<&str>,
) -> RuleDecision {
    let framework_ids = context.framework_ids();
    let role_ids = context.role_ids();
    let paradigm_ids = context.paradigm_ids();
    let runtime_ids = context.runtime_ids();

    decide(&RuleMatchContext {
        rule_id,
        languages: language_ids,
        frameworks: &framework_ids,
        roles: &role_ids,
        paradigms: &paradigm_ids,
        runtimes: &runtime_ids,
        is_test: context.is_test,
        is_low_signal,
        signal,
        base_severity,
        path,
    })
}

fn decide_with_context_trace(
    rule_id: &str,
    language_ids: &[&str],
    context: &AuditContext,
    is_low_signal: bool,
    base_severity: Severity,
    signal: Option<&str>,
    path: Option<&str>,
) -> RuleDecisionTrace {
    let framework_ids = context.framework_ids();
    let role_ids = context.role_ids();
    let paradigm_ids = context.paradigm_ids();
    let runtime_ids = context.runtime_ids();
    decide_with_trace(&RuleMatchContext {
        rule_id,
        languages: language_ids,
        frameworks: &framework_ids,
        roles: &role_ids,
        paradigms: &paradigm_ids,
        runtimes: &runtime_ids,
        is_test: context.is_test,
        is_low_signal,
        signal,
        base_severity,
        path,
    })
}

pub fn apply_file_decision(
    rule_id: &str,
    file: &FileFacts,
    mut finding: Finding,
    signal: Option<&str>,
) -> Option<Finding> {
    let base_severity = finding.severity;
    let decision = decide_for_file(rule_id, file, base_severity, signal);
    if decision.is_suppressed() {
        return None;
    }
    record_decision_provenance(&mut finding, base_severity, signal, &decision);
    finding.severity = decision.severity;
    Some(finding)
}

pub fn decide_for_project(
    rule_id: &str,
    facts: &ScanFacts,
    base_severity: Severity,
    signal: Option<&str>,
) -> RuleDecision {
    let mut language_ids = language_ids_for_project(facts);
    dedup_static_ids(&mut language_ids);
    let framework_ids = framework_ids_for_project(facts);
    decide(&RuleMatchContext {
        rule_id,
        languages: &language_ids,
        frameworks: &framework_ids,
        roles: &[],
        paradigms: &[],
        runtimes: &[],
        is_test: false,
        is_low_signal: false,
        signal,
        base_severity,
        path: None,
    })
}

pub fn apply_project_decisions(facts: &ScanFacts, findings: Vec<Finding>) -> Vec<Finding> {
    findings
        .into_iter()
        .filter_map(|mut finding| {
            let base_severity = finding.severity;
            let decision = decide_for_project(&finding.rule_id, facts, base_severity, None);
            if decision.is_suppressed() {
                return None;
            }
            record_decision_provenance(&mut finding, base_severity, None, &decision);
            finding.severity = decision.severity;
            Some(finding)
        })
        .collect()
}

pub fn record_decision_provenance(
    finding: &mut Finding,
    base_severity: Severity,
    signal: Option<&str>,
    decision: &RuleDecision,
) {
    finding.provenance.knowledge_decision = Some(KnowledgeDecisionProvenance {
        base_severity,
        signal: signal.map(str::to_string),
        action: match decision.action {
            RuleDecisionAction::Apply => KnowledgeDecisionAction::Apply,
            RuleDecisionAction::Suppress => KnowledgeDecisionAction::Suppress,
            RuleDecisionAction::Downgrade => KnowledgeDecisionAction::Downgrade,
            RuleDecisionAction::Upgrade => KnowledgeDecisionAction::Upgrade,
        },
        decided_severity: decision.severity,
        reason: decision.reason.clone(),
        overlay_applied: decision.via_overlay,
    });
}
