pub(super) fn render_risk_section(summary: &ScanSummary, stats: &ReportStats) -> String {
    let severity_items = severity_order()
        .iter()
        .filter_map(|severity| {
            let count = stats.severity_count(*severity);
            (count > 0).then(|| {
                format!(
                    r#"<li class="pill"><span class="badge {}">{}</span> {}</li>"#,
                    severity.lowercase_label(),
                    severity.label(),
                    count
                )
            })
        })
        .collect::<Vec<_>>();

    let category_items = stats
        .category_counts
        .iter()
        .map(|count| {
            format!(
                r#"<li class="pill">{} {}</li>"#,
                escape_html(&count.label),
                count.count
            )
        })
        .collect::<Vec<_>>();

    let severity = if severity_items.is_empty() {
        "<p class=\"empty\">No findings found.</p>".to_string()
    } else {
        format!(
            r#"<ul class="inline-list">{}</ul>"#,
            severity_items.join("")
        )
    };
    let categories = if category_items.is_empty() {
        String::new()
    } else {
        format!(
            r#"<h3>Categories</h3><ul class="inline-list">{}</ul>"#,
            category_items.join("")
        )
    };

    let hidden_note = if summary.metrics.hidden_suggestions_count > 0 {
        render_hidden_suggestions(summary)
    } else {
        String::new()
    };

    format!(
        r#"<section class="panel"><h2>Risk Summary</h2>{hidden_note}<h3>Severity</h3>{severity}{categories}</section>"#
    )
}

fn render_hidden_suggestions(summary: &ScanSummary) -> String {
    let mut output = format!(
        r#"<p class="meta">{} strict-only suggestions hidden. Run with <code>--profile strict</code> or <code>--include-maintainability</code> to view.</p>"#,
        summary.metrics.hidden_suggestions_count
    );

    if summary.artifacts.hidden_suggestions.is_empty() {
        return output;
    }

    let rows = summary.artifacts.hidden_suggestions
        .iter()
        .take(8)
        .map(|item| {
            format!(
                "<tr><td><code>{}</code></td><td><code>{}</code></td><td><code>{}</code></td><td class=\"num-cell\">{}</td><td>{}</td></tr>",
                escape_html(&item.category),
                escape_html(&item.intent),
                escape_html(&item.rule_id),
                item.count,
                escape_html(&item.reason)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    output.push_str(&format!(
        "<h3>Top Hidden Suggestions</h3><table><thead><tr><th>Category</th><th>Intent</th><th>Rule</th><th class=\"num-cell\">Count</th><th>Reason</th></tr></thead><tbody>{rows}</tbody></table>"
    ));

    if summary.artifacts.hidden_suggestions.len() > 8 {
        output.push_str(&format!(
            r#"<p class="meta">{} more hidden group(s).</p>"#,
            summary.artifacts.hidden_suggestions.len() - 8
        ));
    }

    output
}

pub(super) fn render_top_rules_section(stats: &ReportStats) -> String {
    if stats.top_rules.is_empty() {
        return "<section class=\"panel\"><h2>Top Rules</h2><p class=\"empty\">No rules triggered.</p></section>".to_string();
    }

    let rows = stats
        .top_rules
        .iter()
        .map(|rule| {
            let severity = rule.severity.unwrap_or(Severity::Info);
            format!(
                "<tr><td><code>{}</code></td><td class=\"num-cell\">{}</td><td><span class=\"badge {}\">{}</span></td></tr>",
                escape_html(&rule.label),
                rule.count,
                severity.lowercase_label(),
                severity.label()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "<section><h2>Top Rules</h2><table><thead><tr><th>Rule</th><th class=\"num-cell\">Count</th><th>Max severity</th></tr></thead><tbody>{rows}</tbody></table></section>"
    )
}

pub(super) fn render_filter_bar(stats: &ReportStats) -> String {
    if stats.total_findings == 0 {
        return String::new();
    }

    let mut chips = vec![
        r#"<button class="filter-chip clear" type="button" data-filter-clear>Clear filters</button>"#
            .to_string(),
    ];

    for severity in severity_order() {
        let count = stats.severity_count(severity);
        if count > 0 {
            chips.push(filter_chip(
                "severity",
                severity.lowercase_label(),
                &format!("{} ({count})", severity.label()),
            ));
        }
    }

    for category in &stats.category_counts {
        chips.push(filter_chip(
            "category",
            &category.label,
            &format!("{} ({})", category.label, category.count),
        ));
    }

    for rule in &stats.top_rules {
        chips.push(filter_chip(
            "rule",
            &rule.label,
            &format!("{} ({})", rule.label, rule.count),
        ));
    }

    format!(r#"<div class="filters">{}</div>"#, chips.join("\n"))
}
