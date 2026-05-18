use super::escape::escape_html;
use crate::baseline::diff::BaselineScanReport;
use crate::baseline::gate::CiGateResult;
use crate::findings::types::Severity;
use crate::output::report_stats::{ReportStats, severity_order};
use crate::scan::types::ScanSummary;

mod frameworks;

pub(super) fn render_scan_meta(summary: &ScanSummary) -> String {
    if summary.mode == crate::scan::types::ScanMode::Changed {
        let base = summary
            .base_ref
            .as_ref()
            .map(|base| format!(" since <code>{}</code>", escape_html(base)))
            .unwrap_or_else(|| " against <code>HEAD</code>".to_string());
        let mut output = format!(
            r#"<p class="meta">Scope: changed files{base}; {} changed file(s); repo-level rules skipped.</p>"#,
            summary.changed_files_count
        );
        if let Some(cache) = &summary.cache_telemetry {
            output.push_str(&format!(
                r#"<p class="meta">Cache: {} hit(s), {} miss(es), {} skipped, {}% hit rate.</p>"#,
                cache.hits, cache.misses, cache.skipped, cache.hit_rate_percent
            ));
        }
        return output;
    }

    r#"<p class="meta">Scope: full scan; repo-level rules included.</p>"#.to_string()
}

pub(super) fn render_summary_cards(summary: &ScanSummary, stats: &ReportStats) -> String {
    let mut cards = vec![
        summary_card(stats.risk_label, "Risk"),
        summary_card(format!("{}/100", stats.health_score), "Health"),
        summary_card(stats.total_findings, "Visible Findings"),
        summary_card(summary.files_count, "Files"),
        summary_card(summary.lines_of_code, "Lines of Code"),
        summary_card(format!("{:.1}/kloc", stats.finding_density), "Density"),
    ];

    if summary.hidden_suggestions_count > 0 {
        cards.push(summary_card(
            summary.hidden_suggestions_count,
            "Hidden Suggestions",
        ));
    }

    if summary.skipped_files_count > 0 {
        cards.push(summary_card(summary.skipped_files_count, "Skipped"));
    }

    cards.join("\n  ")
}

pub(super) fn render_baseline_summary_cards(
    report: &BaselineScanReport,
    stats: &ReportStats,
) -> String {
    let mut cards = vec![
        summary_card(stats.risk_label, "Risk"),
        summary_card(format!("{}/100", stats.health_score), "Health"),
        summary_card(report.summary.findings.len(), "Visible Findings"),
        summary_card(report.new_count(), "New"),
        summary_card(report.existing_count(), "Existing"),
        summary_card(report.summary.files_count, "Files"),
    ];

    if report.summary.hidden_suggestions_count > 0 {
        cards.push(summary_card(
            report.summary.hidden_suggestions_count,
            "Hidden Suggestions",
        ));
    }

    if report.summary.skipped_files_count > 0 {
        cards.push(summary_card(report.summary.skipped_files_count, "Skipped"));
    }

    cards.join("\n  ")
}

pub(super) fn render_baseline_meta(
    report: &BaselineScanReport,
    ci_gate: Option<&CiGateResult>,
) -> String {
    let baseline = match &report.baseline_path {
        Some(path) => format!(
            "Baseline: <code>{}</code>",
            escape_html(&path.to_string_lossy())
        ),
        None => "Baseline: none (all findings treated as new)".to_string(),
    };
    let gate = ci_gate
        .map(|ci_gate| {
            let status = if ci_gate.passed() { "passed" } else { "failed" };
            format!(" CI gate: {status} ({})", escape_html(&ci_gate.label()))
        })
        .unwrap_or_default();

    format!(r#"<p class="meta">{baseline}.{gate}</p>"#)
}

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

    let hidden_note = if summary.hidden_suggestions_count > 0 {
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
        summary.hidden_suggestions_count
    );

    if summary.hidden_suggestions.is_empty() {
        return output;
    }

    let rows = summary
        .hidden_suggestions
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

    if summary.hidden_suggestions.len() > 8 {
        output.push_str(&format!(
            r#"<p class="meta">{} more hidden group(s).</p>"#,
            summary.hidden_suggestions.len() - 8
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

pub(super) fn render_languages_section(summary: &ScanSummary) -> String {
    if summary.languages.is_empty() {
        return "<p class=\"empty\">No languages detected.</p>".to_string();
    }

    let rows = summary
        .languages
        .iter()
        .map(|language| {
            format!(
                "<tr><td>{}</td><td class=\"num-cell\">{}</td></tr>",
                escape_html(&language.name),
                language.files_count
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "<table><thead><tr><th>Language</th><th class=\"num-cell\">Files</th></tr></thead><tbody>{rows}</tbody></table>"
    )
}

pub(super) fn render_frameworks_section(summary: &ScanSummary) -> String {
    frameworks::render_frameworks_section(summary)
}

fn summary_card(value: impl ToString, label: &str) -> String {
    format!(
        r#"<div class="card"><div class="num">{}</div><div class="label">{}</div></div>"#,
        escape_html(&value.to_string()),
        escape_html(label)
    )
}

fn filter_chip(filter_type: &str, value: &str, label: &str) -> String {
    format!(
        r#"<button class="filter-chip" type="button" data-filter-type="{}" data-filter-value="{}">{}</button>"#,
        escape_html(filter_type),
        escape_html(value),
        escape_html(label)
    )
}
