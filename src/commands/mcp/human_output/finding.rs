use serde_json::Value;
use std::fmt::Write as FmtWrite;
use std::path::Path;

pub(super) fn render_finding(report: &Value) -> String {
    let mut output = String::from("# RepoPilot finding explanation\n\n");
    let root = text(report.get("source_root")).unwrap_or("");
    let status = text(report.get("status"))
        .or_else(|| text(report.pointer("/replay/status")))
        .unwrap_or("explained");
    let _ = writeln!(output, "**Status:** {status}");
    if let Some(finding) = report.get("finding") {
        render_summary(&mut output, finding, root);
    }
    render_guidance(&mut output, report, root);
    render_rule_decision(&mut output, report.pointer("/explanation/decision"));
    render_verification(
        &mut output,
        report.pointer("/decision/verification_plan/steps"),
    );
    render_replay(&mut output, report.get("replay"));
    render_candidates(&mut output, report.get("candidates"));
    render_limitations(&mut output, report.get("limitations"));
    output
}

fn render_summary(output: &mut String, finding: &Value, root: &str) {
    let _ = writeln!(
        output,
        "**Finding:** {} · {} · `{}`",
        text(finding.get("severity")).unwrap_or("unknown severity"),
        one_line(text(finding.get("title")).unwrap_or("finding")),
        code(text(finding.get("rule_id")).unwrap_or("unknown rule"))
    );
    if let Some(location) = location(finding, root) {
        let _ = writeln!(output, "**Location:** `{}`", code(&location));
    }
    if let Some(description) = text(finding.get("description")) {
        let _ = writeln!(output, "\n{}", localize(description, root));
    }
    if let Some(recommendation) = text(finding.get("recommendation")) {
        let _ = writeln!(output, "\n**Recommendation:** {}", one_line(recommendation));
    }
}

fn render_guidance(output: &mut String, report: &Value, root: &str) {
    let Some(guidance) = report.pointer("/decision/explanation") else {
        if let Some(message) = text(report.get("message")) {
            let _ = writeln!(output, "\n{}", one_line(message));
        }
        return;
    };
    if let Some(claim) = text(guidance.get("claim")) {
        let _ = writeln!(
            output,
            "\n**Why it was reported:** {}",
            localize(claim, root)
        );
    }
    render_evidence_basis(output, guidance.get("evidence_basis"));
    if let Some(next) = text(guidance.get("next_action")) {
        let _ = writeln!(output, "\n**Next step:** {}", one_line(next));
    }
    render_limitations(output, guidance.get("limitations"));
}

fn render_evidence_basis(output: &mut String, basis: Option<&Value>) {
    let Some(basis) = basis else { return };
    let count = basis
        .get("location_count")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    let _ = writeln!(
        output,
        "**Evidence basis:** {} · {} scope · {count} location(s)",
        text(basis.get("source")).unwrap_or("unknown source"),
        text(basis.get("scope")).unwrap_or("unknown")
    );
}

fn render_rule_decision(output: &mut String, decision: Option<&Value>) {
    let Some(decision) = decision else { return };
    output.push_str("\n## Rule decision\n");
    let _ = writeln!(
        output,
        "- **Rule:** `{}` · **Signal:** `{}`",
        code(text(decision.get("rule_id")).unwrap_or("unknown")),
        code(text(decision.get("signal")).unwrap_or("general"))
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
    render_trace(output, decision.get("trace"));
}

fn render_trace(output: &mut String, trace: Option<&Value>) {
    let Some(trace) = trace.and_then(Value::as_array) else {
        return;
    };
    for step in trace.iter().take(4) {
        if let Some(reason) = text(step.get("reason")) {
            let _ = writeln!(
                output,
                "- {} [{}]: {}",
                one_line(text(step.get("label")).unwrap_or("Decision step")),
                text(step.get("status")).unwrap_or("recorded"),
                one_line(reason)
            );
        }
    }
}

fn render_verification(output: &mut String, steps: Option<&Value>) {
    let Some(steps) = steps.and_then(Value::as_array) else {
        return;
    };
    if steps.is_empty() {
        return;
    }
    output.push_str("\n## Suggested verification\n");
    for step in steps.iter().take(5).filter_map(Value::as_str) {
        let _ = writeln!(output, "- {}", one_line(step));
    }
}

fn render_replay(output: &mut String, replay: Option<&Value>) {
    let Some(replay) = replay else { return };
    if let Some(reason) = text(replay.get("reason")) {
        let _ = writeln!(output, "\n**Replay note:** {}", one_line(reason));
    }
    if let Some(notes) = replay.get("notes").and_then(Value::as_array) {
        for note in notes.iter().take(4).filter_map(Value::as_str) {
            let _ = writeln!(output, "- {}", one_line(note));
        }
    }
}

fn render_candidates(output: &mut String, candidates: Option<&Value>) {
    let Some(candidates) = candidates.and_then(Value::as_array) else {
        return;
    };
    if candidates.is_empty() {
        return;
    }
    output.push_str("\n## Matching occurrences\n");
    for candidate in candidates.iter().take(10) {
        let location =
            location(candidate, "").unwrap_or_else(|| "location unavailable".to_string());
        let _ = writeln!(
            output,
            "- {} (`{}`)",
            one_line(text(candidate.get("title")).unwrap_or("finding occurrence")),
            code(&location)
        );
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

fn location(report: &Value, root: &str) -> Option<String> {
    let evidence = report
        .get("evidence")
        .and_then(Value::as_array)
        .and_then(|items| items.first());
    let path = text(report.get("path")).or_else(|| text(evidence?.get("path")))?;
    let relative = Path::new(path)
        .strip_prefix(root)
        .map(Path::to_string_lossy)
        .map(|path| path.into_owned())
        .unwrap_or_else(|_| path.to_string());
    let line = report
        .get("line_start")
        .or_else(|| evidence.and_then(|item| item.get("line_start")))
        .and_then(Value::as_u64)
        .map(|line| format!(":{line}"))
        .unwrap_or_default();
    Some(format!("{relative}{line}"))
}

fn text(value: Option<&Value>) -> Option<&str> {
    value
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
}

fn localize(value: &str, root: &str) -> String {
    let mut value = one_line(value);
    if !root.is_empty() {
        value = value.replace(root, ".");
    }
    value
}

fn one_line(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn code(value: &str) -> String {
    value.replace('`', "\\`")
}
