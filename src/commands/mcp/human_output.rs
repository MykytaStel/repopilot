use super::analysis_store::AnalysisKind;
use serde_json::Value;
use std::fmt::Write as FmtWrite;
use std::path::Path;

mod explanations;
mod file;
mod finding;
mod signal;
pub(super) use explanations::render_explanation;

pub(super) fn render(kind: AnalysisKind, report: &Value, pagination: Option<&Value>) -> String {
    match kind {
        AnalysisKind::Scan => render_scan(report, pagination),
        AnalysisKind::Review => render_review(report, pagination),
    }
}

fn render_scan(report: &Value, pagination: Option<&Value>) -> String {
    let mut output = String::from("# RepoPilot scan report\n\n");
    let _ = writeln!(output, "**Health:** {}/100", number(report, "health_score"));
    let _ = writeln!(
        output,
        "**Maintainability:** {}/100",
        number(report, "maintainability_score")
    );
    let _ = writeln!(
        output,
        "**Coverage:** {} files analyzed of {} discovered · {} visible findings",
        number(report, "files_analyzed"),
        number(report, "files_discovered"),
        number(report, "visible_findings_count")
    );
    render_pagination(&mut output, pagination);
    render_findings(&mut output, report, false);
    output
}

fn render_review(report: &Value, pagination: Option<&Value>) -> String {
    let mut output = String::from("# RepoPilot change review\n\n");
    let readiness = text(report.pointer("/merge_readiness/verdict")).unwrap_or("unknown");
    let proof = text(report.pointer("/change_proof/verdict")).unwrap_or("unknown");
    let _ = writeln!(output, "**Merge readiness:** {readiness}");
    let _ = writeln!(output, "**Change proof:** {proof}");
    let _ = writeln!(
        output,
        "**Changed files:** {} · **Findings:** {} in changed files, {} elsewhere",
        array_len(report.get("changed_files")),
        number_at(report, "/review/in_diff_findings"),
        number_at(report, "/review/out_of_diff_findings")
    );
    let tiers = report.get("tiered_signals");
    let _ = writeln!(
        output,
        "**Review signals:** {} definitely · {} maybe · {} noise",
        array_len(tiers.and_then(|value| value.get("definitely"))),
        array_len(tiers.and_then(|value| value.get("maybe"))),
        array_len(tiers.and_then(|value| value.get("noise")))
    );
    render_pagination(&mut output, pagination);
    render_reasons(&mut output, report.pointer("/merge_readiness/reasons"));
    render_findings(&mut output, report, true);
    output
}

fn render_findings(output: &mut String, report: &Value, review: bool) {
    let findings = report.get("findings").and_then(Value::as_array);
    let displayed = findings
        .into_iter()
        .flatten()
        .filter(|finding| !review || finding.get("in_diff").and_then(Value::as_bool) != Some(false))
        .collect::<Vec<_>>();
    if displayed.is_empty() {
        let message = if review {
            "No findings on changed files in this result."
        } else {
            "No findings in this result."
        };
        let _ = writeln!(output, "\n## Findings\n\n{message}");
        return;
    }
    output.push_str("\n## Findings\n");
    let root = text(report.get("root_path")).unwrap_or("");
    for finding in displayed {
        let severity = text(finding.get("severity")).unwrap_or("unknown");
        let title = one_line(text(finding.get("title")).unwrap_or("Untitled finding"));
        let rule = text(finding.get("rule_id")).unwrap_or("unknown rule");
        let location = finding_location(finding, root);
        let _ = write!(output, "\n- **{severity} · {title}**");
        if let Some(location) = location {
            let _ = write!(output, " (`{}`)", code(&location));
        }
        let _ = writeln!(output, " · `{}`", code(rule));
        if let Some(description) = text(finding.get("description")) {
            let _ = writeln!(output, "  {}", one_line(description));
        }
        if let Some(recommendation) = text(finding.get("recommendation")) {
            let _ = writeln!(output, "  _Next:_ {}", one_line(recommendation));
        }
    }
}

fn render_reasons(output: &mut String, reasons: Option<&Value>) {
    let Some(reasons) = reasons.and_then(Value::as_array) else {
        return;
    };
    if reasons.is_empty() {
        return;
    }
    output.push_str("\n## Why this needs review\n");
    for reason in reasons.iter().take(5) {
        if let Some(message) = text(reason.get("message")) {
            let _ = writeln!(output, "- {}", one_line(message));
        }
    }
}

fn render_pagination(output: &mut String, pagination: Option<&Value>) {
    let Some(pagination) = pagination else {
        return;
    };
    let _ = writeln!(
        output,
        "**Page:** findings {}–{} of {}{}",
        number(pagination, "offset").saturating_add(1),
        number(pagination, "offset").saturating_add(number(pagination, "returned")),
        number(pagination, "total"),
        pagination
            .get("next_offset")
            .and_then(Value::as_u64)
            .map(|offset| format!(" · next offset {offset}"))
            .unwrap_or_default()
    );
}

fn finding_location(finding: &Value, root: &str) -> Option<String> {
    let evidence = finding.get("evidence")?.as_array()?.first()?;
    let path = text(evidence.get("path"))?;
    let relative = Path::new(path)
        .strip_prefix(root)
        .map(Path::to_string_lossy)
        .map(|path| path.into_owned())
        .unwrap_or_else(|_| path.to_string());
    let line = evidence.get("line_start").and_then(Value::as_u64)?;
    Some(format!("{relative}:{line}"))
}

fn number(value: &Value, key: &str) -> u64 {
    value.get(key).and_then(Value::as_u64).unwrap_or_default()
}

fn number_at(value: &Value, pointer: &str) -> u64 {
    value
        .pointer(pointer)
        .and_then(Value::as_u64)
        .unwrap_or_default()
}

fn array_len(value: Option<&Value>) -> usize {
    value.and_then(Value::as_array).map_or(0, Vec::len)
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

#[cfg(test)]
mod tests {
    use super::{AnalysisKind, render};
    use serde_json::json;

    #[test]
    fn scan_output_shows_finding_title_and_relative_location() {
        let report = json!({
            "root_path": "/repo",
            "health_score": 90,
            "maintainability_score": 80,
            "files_analyzed": 2,
            "files_discovered": 3,
            "visible_findings_count": 1,
            "findings": [{
                "title": "Possible secret detected",
                "rule_id": "security.secret-candidate",
                "severity": "HIGH",
                "description": "Review this value.",
                "recommendation": "Move it to an environment variable.",
                "evidence": [{"path": "/repo/src/config.ts", "line_start": 4}]
            }]
        });

        let text = render(AnalysisKind::Scan, &report, None);

        assert!(text.starts_with("# RepoPilot scan report"));
        assert!(text.contains("2 files analyzed of 3 discovered"));
        assert!(text.contains("Possible secret detected** (`src/config.ts:4`)"));
        assert!(text.contains("_Next:_ Move it to an environment variable."));
    }

    #[test]
    fn review_output_explains_readiness_and_excludes_out_of_diff_findings() {
        let report = json!({
            "merge_readiness": {"verdict": "review", "reasons": []},
            "change_proof": {"verdict": "REVIEW"},
            "changed_files": [{"path": "src/lib.rs"}],
            "review": {"in_diff_findings": 1, "out_of_diff_findings": 1},
            "tiered_signals": {
                "definitely": [{"signal_id": "def-1"}, {"signal_id": "def-2"}],
                "maybe": [{"signal_id": "maybe-1"}],
                "noise": [{"signal_id": "noise-1"}]
            },
            "findings": [
                {"title": "Changed issue", "rule_id": "test.changed", "severity": "HIGH",
                    "in_diff": true, "evidence": [{"path": "src/lib.rs", "line_start": 3}]},
                {"title": "Existing issue", "rule_id": "test.existing", "severity": "LOW",
                    "in_diff": false, "evidence": [{"path": "src/other.rs", "line_start": 8}]}
            ]
        });

        let text = render(AnalysisKind::Review, &report, None);

        assert!(text.contains("**Merge readiness:** review"));
        assert!(text.contains("**Change proof:** REVIEW"));
        assert!(text.contains("**Review signals:** 2 definitely · 1 maybe · 1 noise"));
        assert!(text.contains("Changed issue"));
        assert!(!text.contains("Existing issue"));
    }

    #[test]
    fn review_output_names_the_empty_changed_finding_state() {
        let report = json!({
            "findings": [{"title": "Existing issue", "in_diff": false}],
            "review": {"in_diff_findings": 0, "out_of_diff_findings": 1}
        });

        let text = render(AnalysisKind::Review, &report, None);

        assert!(text.contains("No findings on changed files in this result."));
    }
}
