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
                language.files_analyzed
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
