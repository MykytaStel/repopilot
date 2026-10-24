use serde_json::Value;
use std::fmt::Write as FmtWrite;

pub(super) fn render_file(report: &Value) -> String {
    let mut output = String::from("# RepoPilot file explanation\n\n");
    let context = report.get("context").unwrap_or(&Value::Null);
    let _ = writeln!(
        output,
        "**File:** `{}`",
        code(text(report.get("path")).unwrap_or("unknown file"))
    );
    let _ = writeln!(
        output,
        "**Language:** {} · **Roles:** {}",
        text(context.get("language")).unwrap_or("unknown"),
        string_list(context.get("roles"))
    );
    render_code_type(&mut output, context);
    render_scope(&mut output, report.get("scope"));
    render_role_evidence(&mut output, context.get("role_evidence"));
    render_decision(&mut output, report.get("decision"));
    output
}

fn render_code_type(output: &mut String, context: &Value) {
    let test = context.get("is_test").and_then(Value::as_bool) == Some(true);
    let production = context.get("is_production_code").and_then(Value::as_bool) == Some(true);
    let _ = writeln!(
        output,
        "**Code type:** {} · {}",
        if test { "test" } else { "not marked as test" },
        if production {
            "production code"
        } else {
            "not marked as production code"
        }
    );
}

fn render_scope(output: &mut String, scope: Option<&Value>) {
    let Some(scope) = scope else { return };
    let _ = writeln!(
        output,
        "**Analysis scope:** {} · **Visibility profile:** {}",
        text(scope.get("analysis_scope")).unwrap_or("unknown"),
        text(scope.get("visibility_profile")).unwrap_or("unknown")
    );
    if let Some(note) = text(scope.get("note")) {
        let _ = writeln!(output, "\n{}", one_line(note));
    }
}

fn render_role_evidence(output: &mut String, evidence: Option<&Value>) {
    let Some(evidence) = evidence.and_then(Value::as_array) else {
        return;
    };
    if evidence.is_empty() {
        return;
    }
    output.push_str("\n## Role evidence\n");
    for item in evidence.iter().take(8) {
        let _ = writeln!(
            output,
            "- **{}** ({}): {}",
            text(item.get("role")).unwrap_or("role"),
            text(item.get("source")).unwrap_or("source"),
            one_line(text(item.get("reason")).unwrap_or(""))
        );
    }
}

fn render_decision(output: &mut String, decision: Option<&Value>) {
    let Some(decision) = decision else { return };
    output.push_str("\n## Rule decision\n");
    let _ = writeln!(
        output,
        "- **Rule:** `{}`",
        code(text(decision.get("rule_id")).unwrap_or("unknown rule"))
    );
    let _ = writeln!(
        output,
        "- **Severity:** {} → {} · **Action:** {}",
        text(decision.get("base_severity")).unwrap_or("unknown"),
        text(decision.get("final_severity")).unwrap_or("unknown"),
        text(decision.get("action")).unwrap_or("unknown")
    );
    if let Some(reason) = text(decision.get("reason")) {
        let _ = writeln!(output, "- **Reason:** {}", one_line(reason));
    }
    if let Some(visible) = decision
        .pointer("/visibility/visible_by_default")
        .and_then(Value::as_bool)
    {
        let _ = writeln!(
            output,
            "- **Default profile:** {}",
            if visible { "visible" } else { "hidden" }
        );
    }
}

fn string_list(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| "unclassified".to_string())
}

fn text(value: Option<&Value>) -> Option<&str> {
    value
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
}

fn one_line(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn code(value: &str) -> String {
    value.replace('`', "\\`")
}
