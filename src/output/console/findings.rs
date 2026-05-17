use crate::findings::types::Finding;
use crate::output::color;
use crate::output::report_stats::{
    category_order, findings_for_category, findings_for_rule, first_location, rule_ids_for_findings,
};
use crate::output::report_text::{category_title, first_sentence};
use std::fmt::Write;

pub(crate) fn render_grouped_findings<F>(output: &mut String, findings: &[Finding], status_for: F)
where
    F: Fn(usize) -> Option<&'static str>,
{
    output.push_str("Findings:\n");

    if findings.is_empty() {
        output.push_str("  none\n");
        return;
    }

    for category in category_order() {
        let category_findings = findings_for_category(findings, &category);
        if category_findings.is_empty() {
            continue;
        }

        writeln!(output, "  {}:", category_title(&category)).unwrap();
        let rules = rule_ids_for_findings(&category_findings);
        for rule_id in rules {
            let rule_findings = findings_for_rule(&category_findings, &rule_id);
            writeln!(output, "    {} ({})", rule_id, rule_findings.len()).unwrap();
            for finding in rule_findings {
                let index = findings
                    .iter()
                    .position(|candidate| std::ptr::eq(candidate, finding))
                    .unwrap_or(0);
                render_finding(output, finding, status_for(index));
            }
        }
        output.push('\n');
    }
}

fn render_finding(output: &mut String, finding: &Finding, status: Option<&str>) {
    let severity = color::severity_label(finding.severity_label());
    writeln!(output, "      [{}] {}", severity, finding.title).unwrap();
    writeln!(output, "        Confidence: {}", finding.confidence_label()).unwrap();
    writeln!(
        output,
        "        Priority: {} (risk {}/100)",
        finding.risk.priority.label(),
        finding.risk.score
    )
    .unwrap();
    if let Some(reasons) = risk_reason_text(finding) {
        writeln!(output, "        Risk signals: {reasons}").unwrap();
    }
    if let Some(status) = status {
        writeln!(output, "        Baseline: {status}").unwrap();
    }
    if let Some(location) = first_location(finding) {
        writeln!(output, "        Location: {location}").unwrap();
    }
    for evidence in &finding.evidence {
        let location = if evidence.line_start > 0 {
            format!("{}:{}", evidence.path.display(), evidence.line_start)
        } else {
            evidence.path.display().to_string()
        };
        let snippet = evidence.snippet.trim();
        if snippet.is_empty() {
            writeln!(output, "        Evidence: {location}").unwrap();
        } else {
            writeln!(output, "        Evidence: {location} - {snippet}").unwrap();
        }
    }
    if !finding.description.is_empty() {
        writeln!(
            output,
            "        {}",
            color::dim(&first_sentence(&finding.description, 120))
        )
        .unwrap();
    }
    writeln!(
        output,
        "        Recommendation: {}",
        first_sentence(finding.recommendation_or_default(), 180)
    )
    .unwrap();
    if let Some(url) = &finding.docs_url {
        writeln!(output, "        Docs: {url}").unwrap();
    }
}

fn risk_reason_text(finding: &Finding) -> Option<String> {
    let reasons = finding
        .risk
        .signals
        .iter()
        .filter(|signal| !signal.id.starts_with("severity."))
        .take(3)
        .map(|signal| format!("{} ({:+})", signal.label, signal.weight))
        .collect::<Vec<_>>();

    (!reasons.is_empty()).then(|| reasons.join(", "))
}
