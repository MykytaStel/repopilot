use crate::findings::types::Finding;
use crate::output::FindingRenderLimit;
use crate::output::color;
use crate::output::report_stats::{
    category_order, first_location, indexed_findings_for_rule, indexed_sorted_findings,
    rule_ids_for_indexed_findings,
};
use crate::output::report_text::{category_title, first_sentence};
use std::fmt::Write;

pub(crate) fn render_grouped_findings<F>(
    output: &mut String,
    findings: &[Finding],
    findings_limit: FindingRenderLimit,
    status_for: F,
) where
    F: Fn(usize) -> Option<&'static str>,
{
    output.push_str("Findings:\n");

    if findings.is_empty() {
        output.push_str("  none\n");
        return;
    }

    let shown = findings_limit.detailed_limit(findings.len());
    let indexed_findings = indexed_sorted_findings(findings)
        .into_iter()
        .take(shown)
        .collect::<Vec<_>>();
    if matches!(findings_limit, FindingRenderLimit::Limit(_)) && shown < findings.len() {
        writeln!(
            output,
            "  showing {shown} of {} findings (--max-findings none shows all)",
            findings.len()
        )
        .unwrap();
    }

    for category in category_order() {
        let category_findings = indexed_findings
            .iter()
            .copied()
            .filter(|(_, finding)| finding.category == category)
            .collect::<Vec<_>>();
        if category_findings.is_empty() {
            continue;
        }

        writeln!(output, "  {}:", category_title(&category)).unwrap();
        let rules = rule_ids_for_indexed_findings(&category_findings);
        for rule_id in rules {
            let rule_findings = indexed_findings_for_rule(&category_findings, &rule_id);
            writeln!(output, "    {} ({})", rule_id, rule_findings.len()).unwrap();
            for (index, finding) in rule_findings {
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
    if let Some(plan) = crate::findings::verification::build_verification_plan(finding) {
        writeln!(output, "        Verification:").unwrap();
        for step in &plan.steps {
            writeln!(output, "          - {step}").unwrap();
        }
    }
    if let Some(url) = &finding.docs_url {
        writeln!(output, "        Docs: {url}").unwrap();
    }
}

fn risk_reason_text(finding: &Finding) -> Option<String> {
    let reasons = finding
        .risk
        .signals
        .iter()
        .take(4)
        .map(|signal| format!("{:+} {}: {}", signal.weight, signal.label, signal.reason))
        .collect::<Vec<_>>();

    (!reasons.is_empty()).then(|| reasons.join(", "))
}
