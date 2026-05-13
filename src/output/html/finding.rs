use super::escape::escape_html;
use crate::findings::types::{Finding, FindingCategory};
use crate::output::report_stats::{
    category_order, findings_for_category, findings_for_rule, first_location, rule_ids_for_findings,
};
use crate::scan::types::ScanSummary;

pub(super) fn render_findings_section<F>(summary: &ScanSummary, status_for: F) -> String
where
    F: Fn(usize) -> Option<&'static str>,
{
    if summary.findings.is_empty() {
        return "<p class=\"empty\">No findings found.</p>".to_string();
    }

    let mut output = String::new();
    for category in category_order() {
        let category_findings = findings_for_category(&summary.findings, &category);
        if category_findings.is_empty() {
            continue;
        }

        output.push_str(&format!(
            r#"<section class="finding-group" data-category="{}"><h3>{}</h3>"#,
            escape_html(category.label()),
            category_title(&category)
        ));

        let rules = rule_ids_for_findings(&category_findings);
        for rule_id in rules {
            let rule_findings = findings_for_rule(&category_findings, &rule_id);
            output.push_str(&format!(
                r#"<section class="rule-group" data-rule="{}"><h4><code>{}</code> ({})</h4>"#,
                escape_html(&rule_id),
                escape_html(&rule_id),
                rule_findings.len()
            ));
            for finding in rule_findings {
                let index = summary
                    .findings
                    .iter()
                    .position(|candidate| std::ptr::eq(candidate, finding))
                    .unwrap_or(0);
                output.push_str(&render_finding_card(finding, status_for(index)));
            }
            output.push_str("</section>");
        }
        output.push_str("</section>");
    }

    output
}

fn render_finding_card(finding: &Finding, status: Option<&str>) -> String {
    let location = first_location(finding)
        .map(|location| {
            format!(
                r#"<p class="finding-meta"><strong>Location:</strong> <code>{}</code></p>"#,
                escape_html(&location)
            )
        })
        .unwrap_or_default();
    let status = status
        .map(|status| {
            format!(
                r#"<span class="status {}">baseline: {}</span>"#,
                escape_html(status),
                escape_html(status)
            )
        })
        .unwrap_or_default();
    let evidence = finding
        .evidence
        .first()
        .map(|evidence| {
            if evidence.snippet.trim().is_empty() {
                String::new()
            } else {
                format!(
                    r#"<pre class="snippet">{}</pre>"#,
                    escape_html(evidence.snippet.trim())
                )
            }
        })
        .unwrap_or_default();
    let docs = finding
        .docs_url
        .as_ref()
        .map(|url| {
            format!(
                r#"<p class="finding-meta"><strong>Docs:</strong> <a href="{}">{}</a></p>"#,
                escape_html(url),
                escape_html(url)
            )
        })
        .unwrap_or_default();

    format!(
        r#"<article class="finding-card" data-severity="{}" data-confidence="{}" data-category="{}" data-rule="{}">
  <div class="finding-title"><span class="badge {}">{}</span><span class="badge confidence">confidence {}</span><strong>{}</strong>{}</div>
  <p class="finding-meta"><strong>Rule:</strong> <code>{}</code></p>
  {}
  {}
  {}
</article>"#,
        finding.severity.lowercase_label(),
        finding.confidence.lowercase_label(),
        finding.category.label(),
        escape_html(&finding.rule_id),
        finding.severity.lowercase_label(),
        finding.severity.label(),
        finding.confidence.label(),
        escape_html(&finding.title),
        status,
        escape_html(&finding.rule_id),
        location,
        evidence,
        docs
    )
}

fn category_title(category: &FindingCategory) -> &'static str {
    match category {
        FindingCategory::Security => "Security",
        FindingCategory::Architecture => "Architecture",
        FindingCategory::Framework => "Framework",
        FindingCategory::CodeQuality => "Code Quality",
        FindingCategory::Testing => "Testing",
    }
}
