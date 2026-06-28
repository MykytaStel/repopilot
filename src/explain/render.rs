use crate::explain::model::{
    ExplainDecision, ExplainDecisionTraceStep, ExplainReport, ExplainRoleEvidence,
};
use crate::output::OutputFormat;

pub fn render_explain_report(
    report: &ExplainReport,
    format: OutputFormat,
) -> Result<String, serde_json::Error> {
    match format {
        OutputFormat::Console => Ok(render_console(report)),
        OutputFormat::Markdown => Ok(render_markdown(report)),
        OutputFormat::Json | OutputFormat::Html | OutputFormat::Sarif => {
            serde_json::to_string_pretty(report)
        }
    }
}

fn render_console(report: &ExplainReport) -> String {
    let mut output = String::new();
    output.push_str("RepoPilot Explain\n\n");
    output.push_str(&format!("File: {}\n", report.path));
    output.push_str(&format!(
        "Language: {}\n",
        report.source.language_name.as_deref().unwrap_or("unknown")
    ));
    output.push_str(&format!(
        "Lines of code: {}\n",
        report.source.non_empty_lines
    ));
    output.push_str(&format!(
        "Inline tests: {}\n\n",
        yes_no(report.source.has_inline_tests)
    ));

    output.push_str("Scope:\n");
    output.push_str(&format!(" Analysis: {}\n", report.scope.analysis_scope));
    output.push_str(&format!(
        " Decision source: {}\n",
        report.scope.decision_source
    ));
    output.push_str(&format!(
        " Visibility profile: {}\n",
        report.scope.visibility_profile
    ));
    output.push_str(&format!(
        " Repository context: {}\n",
        yes_no(report.scope.repository_context_included)
    ));
    output.push_str(&format!(
        " Package manifest context: {}\n",
        yes_no(report.scope.package_manifest_context_included)
    ));
    output.push_str(&format!(
        " Scan configuration: {}\n",
        yes_no(report.scope.scan_configuration_included)
    ));
    output.push_str(&format!(
        " Local feedback: {}\n",
        yes_no(report.scope.local_feedback_included)
    ));
    output.push_str(&format!(
        " Baseline: {}\n",
        yes_no(report.scope.baseline_included)
    ));
    output.push_str(&format!(" Note: {}\n\n", report.scope.note));

    output.push_str("Audit context:\n");
    output.push_str(&format!(" Language: {}\n", report.context.language));
    output.push_str(&format!(
        " Language support: {}\n",
        report
            .context
            .language_support
            .as_deref()
            .unwrap_or("unknown")
    ));
    output.push_str(&format!(
        " Frameworks: {}\n",
        comma_or_none(&report.context.frameworks)
    ));
    output.push_str(&format!(
        " Roles: {}\n",
        comma_or_none(&report.context.roles)
    ));
    render_console_role_evidence(&mut output, &report.context.role_evidence);
    output.push_str(&format!(
        " Paradigms: {}\n",
        comma_or_none(&report.context.paradigms)
    ));
    output.push_str(&format!(
        " Runtimes: {}\n",
        comma_or_none(&report.context.runtimes)
    ));
    output.push_str(&format!(" Test code: {}\n", yes_no(report.context.is_test)));
    output.push_str(&format!(
        " Production code: {}\n",
        yes_no(report.context.is_production_code)
    ));
    if let Some(value) = &report.context.architecture_role {
        output.push_str(&format!(" Architecture role: {}\n", value));
    }
    if let Some(value) = &report.context.module_kind {
        output.push_str(&format!(" Module kind: {}\n", value));
    }
    if let Some(value) = &report.context.language_family {
        output.push_str(&format!(" Language family: {}\n", value));
    }

    output.push_str("\nRule decision:\n");
    match &report.decision {
        Some(decision) => output.push_str(&render_console_decision(decision)),
        None => output.push_str(" not requested; pass --rule <RULE_ID> to evaluate one\n"),
    }
    output
}

fn render_console_role_evidence(output: &mut String, evidence: &[ExplainRoleEvidence]) {
    output.push_str(" Role evidence:\n");
    if evidence.is_empty() {
        output.push_str("  - none\n");
        return;
    }
    for item in evidence {
        output.push_str(&format!(
            "  - {} [{}]: {}\n",
            item.role, item.source, item.reason
        ));
    }
}

fn render_console_decision(decision: &ExplainDecision) -> String {
    let mut output = String::new();
    output.push_str(&format!(" Rule: {}\n", decision.rule_id));
    output.push_str(&format!(
        " Signal: {}\n",
        decision.signal.as_deref().unwrap_or("none")
    ));
    output.push_str(&format!(
        " Base severity: {}\n",
        decision.base_severity.label()
    ));
    output.push_str(&format!(" Action: {}\n", decision.action));
    output.push_str(&format!(
        " Final severity: {}\n",
        decision.final_severity.label()
    ));
    if let Some(reason) = &decision.reason {
        output.push_str(&format!(" Reason: {reason}\n"));
    }
    if let Some(signal) = &decision.risk_signal {
        output.push_str(&format!(
            " Risk signal: {} ({:+}) - {}\n",
            signal.label, signal.weight, signal.reason
        ));
    }
    if let Some(visibility) = &decision.visibility {
        output.push_str(&format!(
            " Visibility: {} in {} profile ({})\n",
            if visibility.visible_by_default {
                "visible"
            } else {
                "hidden"
            },
            visibility.profile,
            visibility.reason
        ));
        output.push_str(&format!(" Intent: {}\n", visibility.intent));
    }

    output.push_str(" Decision trace:\n");
    for step in &decision.trace {
        output.push_str(&format!(
            "  {}. [{} / {}] {}\n",
            step.order, step.stage, step.status, step.label
        ));
        output.push_str(&format!(
            "     Severity: {} -> {}\n",
            step.severity_before.label(),
            step.severity_after.label()
        ));
        if let Some(action) = &step.action {
            output.push_str(&format!("     Action: {action}\n"));
        }
        output.push_str(&format!(
            "     Criteria: {}\n",
            comma_or_none(&step.criteria)
        ));
        output.push_str(&format!("     Reason: {}\n", step.reason));
    }
    output
}

fn render_markdown(report: &ExplainReport) -> String {
    let mut output = String::new();
    output.push_str("# RepoPilot Explain\n\n");
    output.push_str(&format!("- **File:** `{}`\n", report.path));
    output.push_str(&format!(
        "- **Language:** `{}`\n",
        report.source.language_name.as_deref().unwrap_or("unknown")
    ));
    output.push_str(&format!(
        "- **Lines of code:** {}\n",
        report.source.non_empty_lines
    ));
    output.push_str(&format!(
        "- **Inline tests:** {}\n\n",
        yes_no(report.source.has_inline_tests)
    ));

    output.push_str("## Scope\n\n");
    output.push_str(&format!(
        "- **Analysis scope:** `{}`\n",
        report.scope.analysis_scope
    ));
    output.push_str(&format!(
        "- **Decision source:** `{}`\n",
        report.scope.decision_source
    ));
    output.push_str(&format!(
        "- **Visibility profile:** `{}`\n",
        report.scope.visibility_profile
    ));
    output.push_str(&format!(
        "- **Repository context included:** {}\n",
        yes_no(report.scope.repository_context_included)
    ));
    output.push_str(&format!(
        "- **Package manifest context included:** {}\n",
        yes_no(report.scope.package_manifest_context_included)
    ));
    output.push_str(&format!(
        "- **Scan configuration included:** {}\n",
        yes_no(report.scope.scan_configuration_included)
    ));
    output.push_str(&format!(
        "- **Local feedback included:** {}\n",
        yes_no(report.scope.local_feedback_included)
    ));
    output.push_str(&format!(
        "- **Baseline included:** {}\n",
        yes_no(report.scope.baseline_included)
    ));
    output.push_str(&format!("- **Note:** {}\n\n", report.scope.note));

    output.push_str("## Audit context\n\n");
    output.push_str(&format!("- **Language:** `{}`\n", report.context.language));
    output.push_str(&format!(
        "- **Language support:** `{}`\n",
        report
            .context
            .language_support
            .as_deref()
            .unwrap_or("unknown")
    ));
    output.push_str(&format!(
        "- **Frameworks:** {}\n",
        markdown_list(&report.context.frameworks)
    ));
    output.push_str(&format!(
        "- **Roles:** {}\n",
        markdown_list(&report.context.roles)
    ));
    output.push_str(&format!(
        "- **Paradigms:** {}\n",
        markdown_list(&report.context.paradigms)
    ));
    output.push_str(&format!(
        "- **Runtimes:** {}\n",
        markdown_list(&report.context.runtimes)
    ));
    output.push_str(&format!(
        "- **Test code:** {}\n",
        yes_no(report.context.is_test)
    ));
    output.push_str(&format!(
        "- **Production code:** {}\n",
        yes_no(report.context.is_production_code)
    ));
    if let Some(value) = &report.context.architecture_role {
        output.push_str(&format!("- **Architecture role:** `{}`\n", value));
    }
    if let Some(value) = &report.context.module_kind {
        output.push_str(&format!("- **Module kind:** `{}`\n", value));
    }
    if let Some(value) = &report.context.language_family {
        output.push_str(&format!("- **Language family:** `{}`\n", value));
    }

    output.push_str("\n### Role evidence\n\n");
    if report.context.role_evidence.is_empty() {
        output.push_str("None.\n");
    } else {
        for evidence in &report.context.role_evidence {
            output.push_str(&format!(
                "- `{}` from `{}` — {}\n",
                evidence.role, evidence.source, evidence.reason
            ));
        }
    }

    output.push_str("\n## Rule decision\n\n");
    match &report.decision {
        Some(decision) => render_markdown_decision(&mut output, decision),
        None => {
            output.push_str("No rule was requested. Pass `--rule <RULE_ID>` to evaluate one.\n")
        }
    }
    output
}

fn render_markdown_decision(output: &mut String, decision: &ExplainDecision) {
    output.push_str(&format!("- **Rule:** `{}`\n", decision.rule_id));
    output.push_str(&format!(
        "- **Signal:** `{}`\n",
        decision.signal.as_deref().unwrap_or("none")
    ));
    output.push_str(&format!(
        "- **Base severity:** `{}`\n",
        decision.base_severity.label()
    ));
    output.push_str(&format!("- **Action:** `{}`\n", decision.action));
    output.push_str(&format!(
        "- **Final severity:** `{}`\n",
        decision.final_severity.label()
    ));
    if let Some(reason) = &decision.reason {
        output.push_str(&format!("- **Reason:** {reason}\n"));
    }
    if let Some(signal) = &decision.risk_signal {
        output.push_str(&format!(
            "- **Risk signal:** {} ({:+}) — {}\n",
            signal.label, signal.weight, signal.reason
        ));
    }
    if let Some(visibility) = &decision.visibility {
        output.push_str(&format!(
            "- **Visibility:** `{}` in `{}` profile\n",
            if visibility.visible_by_default {
                "visible"
            } else {
                "hidden"
            },
            visibility.profile
        ));
        output.push_str(&format!("- **Intent:** `{}`\n", visibility.intent));
        output.push_str(&format!("- **Visibility reason:** {}\n", visibility.reason));
    }

    output.push_str("\n### Decision trace\n\n");
    output.push_str(
        "| # | Stage | Status | Criteria | Action | Severity | Reason |\n\
         |---:|---|---|---|---|---|---|\n",
    );
    for step in &decision.trace {
        render_markdown_trace_row(output, step);
    }
}

fn render_markdown_trace_row(output: &mut String, step: &ExplainDecisionTraceStep) {
    let criteria = if step.criteria.is_empty() {
        "none".to_string()
    } else {
        step.criteria.join("; ")
    };
    let action = step.action.as_deref().unwrap_or("—");
    output.push_str(&format!(
        "| {} | `{}` | `{}` | {} | `{}` | `{} → {}` | {} |\n",
        step.order,
        markdown_cell(&step.stage),
        markdown_cell(&step.status),
        markdown_cell(&criteria),
        markdown_cell(action),
        step.severity_before.label(),
        step.severity_after.label(),
        markdown_cell(&step.reason)
    ));
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn comma_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values.join(", ")
    }
}

fn markdown_list(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values
            .iter()
            .map(|value| format!("`{value}`"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn markdown_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}
