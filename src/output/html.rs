use crate::scan::types::ScanSummary;

pub fn render(summary: &ScanSummary) -> String {
    let cards = render_summary_cards(summary);
    let languages_section = render_languages_section(summary);
    let findings_section = render_findings_section(summary);

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>RepoPilot Scan Report</title>
<style>
  body {{ font-family: system-ui, sans-serif; margin: 0; padding: 2rem; color: #1a1a1a; background: #f8f8f8; }}
  h1 {{ font-size: 1.6rem; margin-bottom: 0.25rem; }}
  h2 {{ font-size: 1.1rem; margin-top: 2rem; border-bottom: 1px solid #ddd; padding-bottom: 0.3rem; }}
  .meta {{ color: #666; font-size: 0.9rem; margin-bottom: 1.5rem; }}
  .cards {{ display: flex; gap: 1rem; flex-wrap: wrap; margin-bottom: 1.5rem; }}
  .card {{ background: #fff; border: 1px solid #e0e0e0; border-radius: 6px; padding: 1rem 1.5rem; min-width: 120px; }}
  .card .num {{ font-size: 1.8rem; font-weight: 700; }}
  .card .label {{ font-size: 0.75rem; color: #888; text-transform: uppercase; letter-spacing: .05em; }}
  table {{ width: 100%; border-collapse: collapse; background: #fff; border-radius: 6px; overflow: hidden; box-shadow: 0 1px 3px #0001; }}
  th {{ text-align: left; padding: 0.6rem 1rem; background: #f0f0f0; font-size: 0.8rem; text-transform: uppercase; letter-spacing: .05em; }}
  td {{ padding: 0.6rem 1rem; border-top: 1px solid #eee; vertical-align: top; font-size: 0.88rem; }}
  .num {{ text-align: right; }}
  .badge {{ display: inline-block; padding: 0.15rem 0.55rem; border-radius: 3px; font-size: 0.75rem; font-weight: 600; text-transform: uppercase; }}
  .badge.info {{ background: #e8f4ff; color: #2563eb; }}
  .badge.low {{ background: #f0fdf4; color: #16a34a; }}
  .badge.medium {{ background: #fffbeb; color: #d97706; }}
  .badge.high {{ background: #fff7ed; color: #ea580c; }}
  .badge.critical {{ background: #fef2f2; color: #dc2626; }}
  pre.snippet {{ margin: 0.3rem 0 0; font-size: 0.8rem; background: #f5f5f5; padding: 0.4rem 0.6rem; border-radius: 4px; overflow: auto; white-space: pre-wrap; }}
  .empty {{ color: #999; font-style: italic; }}
  .filter-bar {{ display: flex; gap: 0.5rem; flex-wrap: wrap; margin-bottom: 0.75rem; }}
  .filter-btn {{ padding: 0.25rem 0.75rem; border: 1px solid #ccc; border-radius: 20px; cursor: pointer; font-size: 0.8rem; background: #fff; }}
  .filter-btn.active {{ background: #1a1a1a; color: #fff; border-color: #1a1a1a; }}
</style>
</head>
<body>
<h1>RepoPilot Scan Report</h1>
<p class="meta">Path: <code>{path}</code></p>

<div class="cards">
  {cards}
</div>

<h2>Languages</h2>
{languages_section}

<h2>Findings</h2>
<div class="filter-bar" id="filter-bar"></div>
{findings_section}

<script>
  const rows = document.querySelectorAll('table#findings tbody tr');
  const bar = document.getElementById('filter-bar');
  const severities = [...new Set([...rows].map(r => r.querySelector('.badge').textContent.trim()))];
  let active = new Set();

  severities.forEach(sev => {{
    const btn = document.createElement('button');
    btn.className = 'filter-btn';
    btn.textContent = sev;
    btn.onclick = () => {{
      if (active.has(sev)) {{ active.delete(sev); btn.classList.remove('active'); }}
      else {{ active.add(sev); btn.classList.add('active'); }}
      rows.forEach(r => {{
        const rowSev = r.querySelector('.badge').textContent.trim();
        r.style.display = active.size === 0 || active.has(rowSev) ? '' : 'none';
      }});
    }};
    bar.appendChild(btn);
  }});
</script>
</body>
</html>"#,
        path = escape_html(&summary.root_path.to_string_lossy()),
        cards = cards,
        languages_section = languages_section,
        findings_section = findings_section,
    )
}

fn render_summary_cards(summary: &ScanSummary) -> String {
    let mut cards = vec![
        summary_card(summary.files_count, "Files"),
        summary_card(summary.directories_count, "Directories"),
        summary_card(summary.lines_of_code, "Lines of Code"),
        summary_card(summary.findings.len(), "Findings"),
    ];

    if summary.skipped_files_count > 0 {
        cards.push(summary_card(summary.skipped_files_count, "Skipped"));
    }

    cards.join("\n  ")
}

fn summary_card(value: usize, label: &str) -> String {
    format!(
        r#"<div class="card"><div class="num">{value}</div><div class="label">{label}</div></div>"#
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
                "<tr><td>{}</td><td class=\"num\">{}</td></tr>",
                escape_html(&language.name),
                language.files_count
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "<table><thead><tr><th>Language</th><th class=\"num\">Files</th></tr></thead><tbody>{rows}</tbody></table>"
    )
}

fn render_findings_section(summary: &ScanSummary) -> String {
    if summary.findings.is_empty() {
        return "<p class=\"empty\">No findings found.</p>".to_string();
    }

    let rows = summary
        .findings
        .iter()
        .map(render_finding_row)
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "<table id=\"findings\"><thead><tr><th>Severity</th><th>Rule</th><th>Title</th><th>Evidence</th></tr></thead><tbody>{rows}</tbody></table>"
    )
}

fn render_finding_row(finding: &crate::findings::types::Finding) -> String {
    let evidence = finding.evidence.first();
    let location = evidence
        .map(|e| {
            format!(
                "<code>{}:{}</code>",
                escape_html(&e.path.to_string_lossy()),
                e.line_start
            )
        })
        .unwrap_or_default();
    let snippet = evidence
        .map(|e| {
            format!(
                "<pre class=\"snippet\">{}</pre>",
                escape_html(e.snippet.trim())
            )
        })
        .unwrap_or_default();
    let severity_class = finding.severity_label().to_lowercase();

    format!(
        "<tr>\
            <td><span class=\"badge {severity_class}\">{}</span></td>\
            <td><code>{}</code></td>\
            <td>{}</td>\
            <td>{location}{snippet}</td>\
         </tr>",
        escape_html(finding.severity_label()),
        escape_html(&finding.rule_id),
        escape_html(&finding.title),
    )
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
