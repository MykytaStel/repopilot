use crate::explain::model::{ExplainDecision, ExplainReport};
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
    if let Some(arch_role) = &report.context.architecture_role {
        output.push_str(&format!(" Architecture role: {}\n", arch_role));
    }
    if let Some(mod_kind) = &report.context.module_kind {
        output.push_str(&format!(" Module kind: {}\n", mod_kind));
    }
    if let Some(lang_family) = &report.context.language_family {
        output.push_str(&format!(" Language family: {}\n", lang_family));
    }

    output.push_str("\nRule decision:\n");
    match &report.decision {
        Some(decision) => output.push_str(&render_console_decision(decision)),
        None => output.push_str(" not requested; pass --rule <RULE_ID> to evaluate one\n"),
    }

    output
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
    if let Some(arch_role) = &report.context.architecture_role {
        output.push_str(&format!("- **Architecture role:** `{}`\n", arch_role));
    }
    if let Some(mod_kind) = &report.context.module_kind {
        output.push_str(&format!("- **Module kind:** `{}`\n", mod_kind));
    }
    if let Some(lang_family) = &report.context.language_family {
        output.push_str(&format!("- **Language family:** `{}`\n\n", lang_family));
    } else {
        output.push_str("\n");
    }

    output.push_str("## Rule decision\n\n");
    match &report.decision {
        Some(decision) => {
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
        }
        None => {
            output.push_str("No rule was requested. Pass `--rule <RULE_ID>` to evaluate one.\n");
        }
    }

    output
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
