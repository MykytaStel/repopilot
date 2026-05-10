use crate::baseline::diff::BaselineScanReport;
use crate::baseline::gate::CiGateResult;
use crate::findings::types::{Finding, FindingCategory, Severity};
use crate::output::report_stats::{
    ReportStats, TOOL_VERSION, build_report_stats, category_order, findings_for_category,
    findings_for_rule, first_location, rule_ids_for_findings, severity_order,
};
use crate::scan::types::ScanSummary;

pub fn render(summary: &ScanSummary) -> String {
    let stats = build_report_stats(summary);
    let cards = render_summary_cards(summary, &stats);
    let risk_section = render_risk_section(&stats);
    let top_rules_section = render_top_rules_section(&stats);
    let languages_section = render_languages_section(summary);
    let frameworks_section = render_frameworks_section(summary);
    let filter_bar = render_filter_bar(&stats);
    let findings_section = render_findings_section(summary, |_| None);
    let path = summary.root_path.to_string_lossy();
    render_document(DocumentParts {
        path: &path,
        baseline_meta: "",
        cards: &cards,
        risk_section: &risk_section,
        top_rules_section: &top_rules_section,
        languages_section: &languages_section,
        frameworks_section: &frameworks_section,
        filter_bar: &filter_bar,
        findings_section: &findings_section,
    })
}

pub fn render_with_baseline(report: &BaselineScanReport, ci_gate: Option<&CiGateResult>) -> String {
    let stats = build_report_stats(&report.summary);
    let cards = render_baseline_summary_cards(report, &stats);
    let risk_section = render_risk_section(&stats);
    let top_rules_section = render_top_rules_section(&stats);
    let languages_section = render_languages_section(&report.summary);
    let frameworks_section = render_frameworks_section(&report.summary);
    let filter_bar = render_filter_bar(&stats);
    let findings_section = render_findings_section(&report.summary, |index| {
        Some(report.finding_status(index).lowercase_label())
    });
    let baseline_meta = render_baseline_meta(report, ci_gate);

    let path = report.summary.root_path.to_string_lossy();
    render_document(DocumentParts {
        path: &path,
        baseline_meta: &baseline_meta,
        cards: &cards,
        risk_section: &risk_section,
        top_rules_section: &top_rules_section,
        languages_section: &languages_section,
        frameworks_section: &frameworks_section,
        filter_bar: &filter_bar,
        findings_section: &findings_section,
    })
}

struct DocumentParts<'a> {
    path: &'a str,
    baseline_meta: &'a str,
    cards: &'a str,
    risk_section: &'a str,
    top_rules_section: &'a str,
    languages_section: &'a str,
    frameworks_section: &'a str,
    filter_bar: &'a str,
    findings_section: &'a str,
}

fn render_document(p: DocumentParts<'_>) -> String {
    let DocumentParts {
        path,
        baseline_meta,
        cards,
        risk_section,
        top_rules_section,
        languages_section,
        frameworks_section,
        filter_bar,
        findings_section,
    } = p;
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>RepoPilot Scan Report</title>
<style>
  :root {{ color-scheme: light; }}
  body {{ font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; margin: 0; color: #18202f; background: #f6f7f9; }}
  main {{ max-width: 1180px; margin: 0 auto; padding: 28px; }}
  header {{ margin-bottom: 24px; }}
  h1 {{ font-size: 1.75rem; margin: 0 0 0.35rem; letter-spacing: 0; }}
  h2 {{ font-size: 1.05rem; margin: 28px 0 12px; }}
  h3 {{ font-size: 0.98rem; margin: 18px 0 10px; }}
  h4 {{ font-size: 0.92rem; margin: 14px 0 8px; }}
  .meta {{ color: #5f6b7a; font-size: 0.9rem; margin: 0.25rem 0; }}
  .cards {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 10px; margin: 18px 0 20px; }}
  .card {{ background: #fff; border: 1px solid #dde2ea; border-radius: 8px; padding: 14px 16px; }}
  .card .num {{ font-size: 1.45rem; line-height: 1.1; font-weight: 750; }}
  .card .label {{ margin-top: 5px; color: #667085; font-size: 0.74rem; text-transform: uppercase; letter-spacing: .04em; }}
  .panel {{ background: #fff; border: 1px solid #dde2ea; border-radius: 8px; padding: 14px 16px; margin-bottom: 12px; }}
  .inline-list {{ display: flex; flex-wrap: wrap; gap: 8px; margin: 0; padding: 0; list-style: none; }}
  .pill {{ display: inline-flex; align-items: center; gap: 5px; border: 1px solid #d5dbe5; border-radius: 999px; padding: 3px 9px; font-size: 0.82rem; background: #fff; }}
  .badge {{ display: inline-block; padding: 0.15rem 0.5rem; border-radius: 999px; font-size: 0.72rem; font-weight: 700; text-transform: uppercase; }}
  .badge.info {{ background: #e8f4ff; color: #155eef; }}
  .badge.low {{ background: #ecfdf3; color: #067647; }}
  .badge.medium {{ background: #fffaeb; color: #b54708; }}
  .badge.high {{ background: #fff4ed; color: #c4320a; }}
  .badge.critical {{ background: #fef3f2; color: #b42318; }}
  .status {{ font-size: 0.72rem; font-weight: 700; text-transform: uppercase; }}
  .status.new {{ color: #b42318; }}
  .status.existing {{ color: #067647; }}
  table {{ width: 100%; border-collapse: collapse; background: #fff; border: 1px solid #dde2ea; border-radius: 8px; overflow: hidden; }}
  th {{ text-align: left; padding: 0.6rem 0.75rem; background: #eef1f5; color: #475467; font-size: 0.76rem; text-transform: uppercase; letter-spacing: .04em; }}
  td {{ padding: 0.62rem 0.75rem; border-top: 1px solid #edf0f4; vertical-align: top; font-size: 0.88rem; }}
  .num-cell {{ text-align: right; }}
  .filters {{ display: flex; flex-wrap: wrap; gap: 8px; margin-bottom: 14px; }}
  .filter-chip {{ border: 1px solid #cfd6e0; background: #fff; border-radius: 999px; color: #2d3748; cursor: pointer; font: inherit; font-size: 0.82rem; padding: 5px 10px; }}
  .filter-chip.active {{ background: #1f2937; border-color: #1f2937; color: #fff; }}
  .filter-chip.clear {{ color: #475467; }}
  .finding-group {{ margin-bottom: 20px; }}
  .rule-group {{ margin: 12px 0 18px; }}
  .finding-card {{ background: #fff; border: 1px solid #dde2ea; border-radius: 8px; margin: 8px 0; padding: 12px 14px; }}
  .finding-title {{ display: flex; align-items: center; gap: 8px; flex-wrap: wrap; margin-bottom: 8px; }}
  .finding-title strong {{ font-size: 0.95rem; }}
  .finding-meta {{ color: #5f6b7a; font-size: 0.84rem; margin: 4px 0; }}
  pre.snippet {{ margin: 8px 0 0; font-size: 0.8rem; background: #f3f5f7; padding: 8px 10px; border-radius: 6px; overflow: auto; white-space: pre-wrap; }}
  .empty {{ color: #667085; font-style: italic; }}
  code {{ font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: 0.9em; }}
</style>
</head>
<body>
<main>
<header>
  <h1>RepoPilot Scan Report</h1>
  <p class="meta">RepoPilot version: <strong>{version}</strong></p>
  <p class="meta">Path: <code>{path}</code></p>
  {baseline_meta}
</header>

<section class="cards">
  {cards}
</section>

{risk_section}
{top_rules_section}

<h2>Languages</h2>
{languages_section}
{frameworks_section}

<h2>Findings</h2>
{filter_bar}
{findings_section}
</main>

<script>
  const filters = {{
    severity: new Set(),
    category: new Set(),
    rule: new Set(),
  }};

  function matchesFilters(card) {{
    return Object.entries(filters).every(([type, selected]) => {{
      return selected.size === 0 || selected.has(card.dataset[type]);
    }});
  }}

  function refreshFindings() {{
    document.querySelectorAll('.finding-card').forEach(card => {{
      card.hidden = !matchesFilters(card);
    }});
    document.querySelectorAll('.finding-group, .rule-group').forEach(group => {{
      const cards = [...group.querySelectorAll('.finding-card')];
      group.hidden = cards.length > 0 && cards.every(card => card.hidden);
    }});
  }}

  document.querySelectorAll('[data-filter-type]').forEach(button => {{
    button.addEventListener('click', () => {{
      const type = button.dataset.filterType;
      const value = button.dataset.filterValue;
      if (filters[type].has(value)) {{
        filters[type].delete(value);
        button.classList.remove('active');
      }} else {{
        filters[type].add(value);
        button.classList.add('active');
      }}
      refreshFindings();
    }});
  }});

  document.querySelector('[data-filter-clear]')?.addEventListener('click', () => {{
    Object.values(filters).forEach(set => set.clear());
    document.querySelectorAll('.filter-chip.active').forEach(button => button.classList.remove('active'));
    refreshFindings();
  }});
</script>
</body>
</html>"#,
        version = TOOL_VERSION,
        path = escape_html(path),
        baseline_meta = baseline_meta,
        cards = cards,
        risk_section = risk_section,
        top_rules_section = top_rules_section,
        languages_section = languages_section,
        frameworks_section = frameworks_section,
        filter_bar = filter_bar,
        findings_section = findings_section,
    )
}

fn render_summary_cards(summary: &ScanSummary, stats: &ReportStats) -> String {
    let mut cards = vec![
        summary_card(stats.risk_label, "Risk"),
        summary_card(format!("{}/100", stats.health_score), "Health"),
        summary_card(stats.total_findings, "Findings"),
        summary_card(summary.files_count, "Files"),
        summary_card(summary.lines_of_code, "Lines of Code"),
        summary_card(format!("{:.1}/kloc", stats.finding_density), "Density"),
    ];

    if summary.skipped_files_count > 0 {
        cards.push(summary_card(summary.skipped_files_count, "Skipped"));
    }

    cards.join("\n  ")
}

fn render_baseline_summary_cards(report: &BaselineScanReport, stats: &ReportStats) -> String {
    let mut cards = vec![
        summary_card(stats.risk_label, "Risk"),
        summary_card(format!("{}/100", stats.health_score), "Health"),
        summary_card(report.summary.findings.len(), "Findings"),
        summary_card(report.new_count(), "New"),
        summary_card(report.existing_count(), "Existing"),
        summary_card(report.summary.files_count, "Files"),
    ];

    if report.summary.skipped_files_count > 0 {
        cards.push(summary_card(report.summary.skipped_files_count, "Skipped"));
    }

    cards.join("\n  ")
}

fn summary_card(value: impl ToString, label: &str) -> String {
    format!(
        r#"<div class="card"><div class="num">{}</div><div class="label">{}</div></div>"#,
        escape_html(&value.to_string()),
        escape_html(label)
    )
}

fn render_baseline_meta(report: &BaselineScanReport, ci_gate: Option<&CiGateResult>) -> String {
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

fn render_risk_section(stats: &ReportStats) -> String {
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

    format!(
        r#"<section class="panel"><h2>Risk Summary</h2><h3>Severity</h3>{severity}{categories}</section>"#
    )
}

fn render_top_rules_section(stats: &ReportStats) -> String {
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

fn render_filter_bar(stats: &ReportStats) -> String {
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

fn filter_chip(filter_type: &str, value: &str, label: &str) -> String {
    format!(
        r#"<button class="filter-chip" type="button" data-filter-type="{}" data-filter-value="{}">{}</button>"#,
        escape_html(filter_type),
        escape_html(value),
        escape_html(label)
    )
}

fn render_languages_section(summary: &ScanSummary) -> String {
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

fn render_frameworks_section(summary: &ScanSummary) -> String {
    if summary.detected_frameworks.is_empty()
        && summary.framework_projects.is_empty()
        && summary.react_native.is_none()
    {
        return String::new();
    }

    let labels: Vec<String> = summary
        .detected_frameworks
        .iter()
        .map(|f| format!(r#"<li class="pill">{}</li>"#, escape_html(&f.label())))
        .collect();
    let mut output = String::from("<h2>Frameworks</h2>");
    if !labels.is_empty() {
        output.push_str(&format!(
            r#"<ul class="inline-list">{}</ul>"#,
            labels.join("")
        ));
    }

    let nested_projects: Vec<_> = summary
        .framework_projects
        .iter()
        .filter(|project| project.path.as_path() != std::path::Path::new("."))
        .collect();
    if !nested_projects.is_empty() {
        output.push_str("<h3>Framework Projects</h3><table><thead><tr><th>Path</th><th>Frameworks</th></tr></thead><tbody>");
        for project in nested_projects {
            let frameworks = project
                .frameworks
                .iter()
                .map(|f| escape_html(&f.label()))
                .collect::<Vec<_>>()
                .join(", ");
            output.push_str(&format!(
                "<tr><td><code>{}</code></td><td>{}</td></tr>",
                escape_html(&project.path.to_string_lossy()),
                frameworks
            ));
        }
        output.push_str("</tbody></table>");
    }

    if let Some(rn) = &summary.react_native {
        output.push_str(&format!(
            "<div class=\"panel\"><h3>React Native</h3><p class=\"meta\">Version {} | Android New Architecture {} | iOS New Architecture {} | Hermes {} | Codegen {}</p></div>",
            escape_html(rn.react_native_version.as_deref().unwrap_or("unknown")),
            escape_html(format_tristate(rn.android_new_arch_enabled)),
            escape_html(format_tristate(rn.ios_new_arch_enabled)),
            escape_html(format_tristate(rn.hermes_enabled)),
            if rn.has_codegen_config { "found" } else { "missing" }
        ));
    }

    output
}

fn render_findings_section<F>(summary: &ScanSummary, status_for: F) -> String
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
        r#"<article class="finding-card" data-severity="{}" data-category="{}" data-rule="{}">
  <div class="finding-title"><span class="badge {}">{}</span><strong>{}</strong>{}</div>
  <p class="finding-meta"><strong>Rule:</strong> <code>{}</code></p>
  {}
  {}
  {}
</article>"#,
        finding.severity.lowercase_label(),
        finding.category.label(),
        escape_html(&finding.rule_id),
        finding.severity.lowercase_label(),
        finding.severity.label(),
        escape_html(&finding.title),
        status,
        escape_html(&finding.rule_id),
        location,
        evidence,
        docs
    )
}

fn format_tristate(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "enabled",
        Some(false) => "disabled",
        None => "unknown",
    }
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

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
