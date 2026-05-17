use crate::baseline::diff::BaselineScanReport;
use crate::findings::types::{Finding, Severity};
use crate::output::render_helpers::escape_table_cell;
use crate::output::report_stats::{
    category_order, findings_for_category, findings_for_rule, first_location, rule_ids_for_findings,
};
use crate::output::report_text::{category_label_rank, category_title, first_sentence};
use std::collections::BTreeMap;
use std::fmt::Write;

pub(crate) fn render_findings_index(
    output: &mut String,
    findings: &[Finding],
    baseline: Option<&BaselineScanReport>,
) {
    output.push_str("## Findings Index\n\n");

    if findings.is_empty() {
        output.push_str("No findings found.\n\n");
        return;
    }

    let rows = grouped_index_rows(findings, baseline);
    if baseline.is_some() {
        output.push_str(
            "| Category | Rule | Max severity | Count | New | Existing | First location |\n",
        );
        output.push_str("| --- | --- | --- | ---: | ---: | ---: | --- |\n");
        for row in rows {
            writeln!(
                output,
                "| {} | `{}` | {} | {} | {} | {} | {} |",
                escape_table_cell(&row.category),
                escape_table_cell(&row.rule_id),
                row.severity.label(),
                row.count,
                row.new_count,
                row.existing_count,
                escape_table_cell(&row.first_location.unwrap_or_else(|| "n/a".to_string()))
            )
            .unwrap();
        }
    } else {
        output.push_str("| Category | Rule | Max severity | Count | First location |\n");
        output.push_str("| --- | --- | --- | ---: | --- |\n");
        for row in rows {
            writeln!(
                output,
                "| {} | `{}` | {} | {} | {} |",
                escape_table_cell(&row.category),
                escape_table_cell(&row.rule_id),
                row.severity.label(),
                row.count,
                escape_table_cell(&row.first_location.unwrap_or_else(|| "n/a".to_string()))
            )
            .unwrap();
        }
    }
    output.push('\n');
}

pub(crate) fn render_grouped_findings<F>(output: &mut String, findings: &[Finding], status_for: F)
where
    F: Fn(usize) -> Option<&'static str>,
{
    output.push_str("## Findings\n\n");

    if findings.is_empty() {
        output.push_str("No findings found.\n");
        return;
    }

    for category in category_order() {
        let category_findings = findings_for_category(findings, &category);
        if category_findings.is_empty() {
            continue;
        }

        writeln!(output, "### {}\n", category_title(&category)).unwrap();
        let rules = rule_ids_for_findings(&category_findings);
        for rule_id in rules {
            let rule_findings = findings_for_rule(&category_findings, &rule_id);
            writeln!(output, "#### `{rule_id}` ({})\n", rule_findings.len()).unwrap();

            for finding in rule_findings {
                let index = findings
                    .iter()
                    .position(|candidate| std::ptr::eq(candidate, finding))
                    .unwrap_or(0);
                render_finding_detail(output, finding, status_for(index));
            }
        }
    }
}

fn render_finding_detail(output: &mut String, finding: &Finding, status: Option<&str>) {
    writeln!(
        output,
        "- **[{}] {}**",
        finding.severity_label(),
        finding.title
    )
    .unwrap();
    writeln!(output, "  - Confidence: {}", finding.confidence_label()).unwrap();
    writeln!(
        output,
        "  - Priority: {} (risk {}/100)",
        finding.risk.priority.label(),
        finding.risk.score
    )
    .unwrap();
    if let Some(reasons) = risk_reason_text(finding) {
        writeln!(output, "  - Risk signals: {reasons}").unwrap();
    }
    if let Some(status) = status {
        writeln!(output, "  - Baseline: {status}").unwrap();
    }
    if let Some(location) = first_location(finding) {
        writeln!(output, "  - Location: `{location}`").unwrap();
    }
    for evidence in &finding.evidence {
        let location = if evidence.line_start > 0 {
            format!("{}:{}", evidence.path.display(), evidence.line_start)
        } else {
            evidence.path.display().to_string()
        };
        let snippet = evidence.snippet.trim();
        if snippet.is_empty() {
            writeln!(output, "  - Evidence: `{location}`").unwrap();
        } else {
            writeln!(
                output,
                "  - Evidence: `{location}` - {}",
                inline_snippet(snippet)
            )
            .unwrap();
        }
    }
    if !finding.description.is_empty() {
        writeln!(
            output,
            "  - Context: {}",
            first_sentence(&finding.description, 180)
        )
        .unwrap();
    }
    writeln!(
        output,
        "  - Recommendation: {}",
        first_sentence(finding.recommendation_or_default(), 220)
    )
    .unwrap();
    if let Some(url) = &finding.docs_url {
        writeln!(output, "  - Docs: {url}").unwrap();
    }
    output.push('\n');
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

struct IndexRow {
    category: String,
    rule_id: String,
    severity: Severity,
    count: usize,
    new_count: usize,
    existing_count: usize,
    first_location: Option<String>,
}

fn grouped_index_rows(
    findings: &[Finding],
    baseline: Option<&BaselineScanReport>,
) -> Vec<IndexRow> {
    let mut rows: BTreeMap<(String, String), IndexRow> = BTreeMap::new();

    for (index, finding) in findings.iter().enumerate() {
        let key = (
            finding.category.label().to_string(),
            finding.rule_id.clone(),
        );
        let row = rows.entry(key).or_insert_with(|| IndexRow {
            category: finding.category.label().to_string(),
            rule_id: finding.rule_id.clone(),
            severity: finding.severity,
            count: 0,
            new_count: 0,
            existing_count: 0,
            first_location: first_location(finding),
        });
        row.count += 1;
        row.severity = row.severity.max(finding.severity);
        if row.first_location.is_none() {
            row.first_location = first_location(finding);
        }
        if let Some(report) = baseline {
            match report.finding_status(index).lowercase_label() {
                "new" => row.new_count += 1,
                "existing" => row.existing_count += 1,
                _ => {}
            }
        }
    }

    let mut rows = rows.into_values().collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        category_label_rank(&left.category)
            .cmp(&category_label_rank(&right.category))
            .then_with(|| right.severity.cmp(&left.severity))
            .then_with(|| right.count.cmp(&left.count))
            .then_with(|| left.rule_id.cmp(&right.rule_id))
    });
    rows
}

fn inline_snippet(snippet: &str) -> String {
    snippet.replace('`', "'")
}
