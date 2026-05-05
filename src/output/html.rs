use crate::scan::types::ScanSummary;

pub fn render(summary: &ScanSummary) -> String {
    let findings_rows = summary
        .findings
        .iter()
        .map(|f| {
            let ev = f
                .evidence
                .first()
                .map(|e| {
                    format!(
                        "<code>{}:{}</code>",
                        escape_html(&e.path.to_string_lossy()),
                        e.line_start
                    )
                })
                .unwrap_or_default();

            let snippet = f
                .evidence
                .first()
                .map(|e| {
                    format!(
                        "<pre class=\"snippet\">{}</pre>",
                        escape_html(e.snippet.trim())
                    )
                })
                .unwrap_or_default();

            let severity_class = f.severity_label().to_lowercase();

            format!(
                "<tr>\
                    <td><span class=\"badge {severity_class}\">{}</span></td>\
                    <td><code>{}</code></td>\
                    <td>{}</td>\
                    <td>{ev}{snippet}</td>\
                 </tr>",
                escape_html(f.severity_label()),
                escape_html(&f.rule_id),
                escape_html(&f.title),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let lang_rows = summary
        .languages
        .iter()
        .map(|l| {
            format!(
                "<tr><td>{}</td><td class=\"num\">{}</td></tr>",
                escape_html(&l.name),
                l.files_count
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let findings_empty = if summary.findings.is_empty() {
        "<p class=\"empty\">No findings found.</p>"
    } else {
        ""
    };

    let lang_empty = if summary.languages.is_empty() {
        "<p class=\"empty\">No languages detected.</p>"
    } else {
        ""
    };

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
  <div class="card"><div class="num">{files}</div><div class="label">Files</div></div>
  <div class="card"><div class="num">{dirs}</div><div class="label">Directories</div></div>
  <div class="card"><div class="num">{loc}</div><div class="label">Lines of Code</div></div>
  <div class="card"><div class="num">{finding_count}</div><div class="label">Findings</div></div>
</div>

<h2>Languages</h2>
{lang_empty}
{lang_table}

<h2>Findings</h2>
{findings_empty}
<div class="filter-bar" id="filter-bar"></div>
{findings_table}

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
        files = summary.files_count,
        dirs = summary.directories_count,
        loc = summary.lines_of_code,
        finding_count = summary.findings.len(),
        lang_empty = lang_empty,
        lang_table = if summary.languages.is_empty() {
            String::new()
        } else {
            format!(
                "<table><thead><tr><th>Language</th><th class=\"num\">Files</th></tr></thead><tbody>{lang_rows}</tbody></table>"
            )
        },
        findings_empty = findings_empty,
        findings_table = if summary.findings.is_empty() {
            String::new()
        } else {
            format!(
                "<table id=\"findings\"><thead><tr><th>Severity</th><th>Rule</th><th>Title</th><th>Evidence</th></tr></thead><tbody>{findings_rows}</tbody></table>"
            )
        },
    )
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
