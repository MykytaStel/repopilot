use serde_json::Value;
use std::fmt::Write as FmtWrite;

pub(super) fn render_signal(report: &Value) -> String {
    let mut output = String::from("# RepoPilot review signal\n\n");
    let signal = report.get("signal").unwrap_or(&Value::Null);
    let line = signal
        .get("line")
        .or_else(|| signal.get("line_start"))
        .and_then(Value::as_u64)
        .map(|line| format!(":{line}"))
        .unwrap_or_default();
    let _ = writeln!(
        output,
        "**Signal:** {} · `{}`",
        one_line(text(signal.get("headline")).unwrap_or("Review signal")),
        code(text(signal.get("kind")).unwrap_or("unknown kind"))
    );
    let _ = writeln!(
        output,
        "**Tier:** {} · **Confidence:** {}",
        text(signal.get("tier")).unwrap_or("see review report"),
        text(signal.get("confidence")).unwrap_or("unknown")
    );
    let _ = writeln!(
        output,
        "**Location:** `{}`",
        code(&format!(
            "{}{line}",
            text(signal.get("path")).unwrap_or("unknown path")
        ))
    );
    if let Some(why) = text(report.get("why_it_matters")) {
        let _ = writeln!(output, "\n**Why it matters:** {}", one_line(why));
    }
    render_impact(&mut output, report.get("impact"));
    render_gate(&mut output, report);
    render_verification(&mut output, report);
    render_limitations(&mut output, report.get("limitations"));
    output
}

fn render_impact(output: &mut String, impact: Option<&Value>) {
    let Some(impact) = impact else { return };
    if let Some(path) = text(impact.get("path")) {
        let _ = writeln!(output, "\n**Affected path:** `{}`", code(path));
    }
    let Some(paths) = impact.get("direct_dependents").and_then(Value::as_array) else {
        return;
    };
    let dependents = paths
        .iter()
        .filter_map(Value::as_str)
        .take(8)
        .map(|path| format!("`{}`", code(path)))
        .collect::<Vec<_>>();
    if !dependents.is_empty() {
        let _ = writeln!(output, "**Direct dependents:** {}", dependents.join(", "));
    }
}

fn render_gate(output: &mut String, report: &Value) {
    let eligible = report.pointer("/gate/eligible").and_then(Value::as_bool) == Some(true);
    let suppressed = report.pointer("/gate/suppressed").and_then(Value::as_bool) == Some(true);
    let _ = writeln!(
        output,
        "\n**Gate:** {} · {}",
        if eligible { "eligible" } else { "not eligible" },
        if suppressed { "suppressed" } else { "active" }
    );
}

fn render_verification(output: &mut String, report: &Value) {
    let Some(steps) = report
        .pointer("/verification_plan/steps")
        .and_then(Value::as_array)
    else {
        return;
    };
    if steps.is_empty() {
        return;
    }
    output.push_str("\n## How to verify\n");
    for step in steps.iter().take(6).filter_map(Value::as_str) {
        let _ = writeln!(output, "- {}", one_line(step));
    }
}

fn render_limitations(output: &mut String, limitations: Option<&Value>) {
    let Some(items) = limitations.and_then(Value::as_array) else {
        return;
    };
    if items.is_empty() {
        return;
    }
    output.push_str("\n## Limits\n");
    for item in items.iter().take(5).filter_map(Value::as_str) {
        let _ = writeln!(output, "- {}", one_line(item));
    }
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
