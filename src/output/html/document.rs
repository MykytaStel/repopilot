use super::assets::{SCRIPT, STYLE};
use super::escape::escape_html;
use crate::output::report_stats::TOOL_VERSION;

pub(super) struct DocumentParts<'a> {
    pub(super) path: &'a str,
    pub(super) baseline_meta: &'a str,
    pub(super) cards: &'a str,
    pub(super) risk_section: &'a str,
    pub(super) top_rules_section: &'a str,
    pub(super) languages_section: &'a str,
    pub(super) frameworks_section: &'a str,
    pub(super) filter_bar: &'a str,
    pub(super) findings_section: &'a str,
}

pub(super) fn render_document(p: DocumentParts<'_>) -> String {
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
<style>{STYLE}</style>
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

<script>{SCRIPT}</script>
</body>
</html>"#,
        version = TOOL_VERSION,
        path = escape_html(path),
    )
}
