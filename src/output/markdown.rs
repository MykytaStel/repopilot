use crate::scan::types::ScanSummary;

pub fn render(summary: &ScanSummary) -> String {
    let mut output = String::new();

    output.push_str("# RepoPilot Scan Report\n\n");

    output.push_str("## Summary\n\n");
    output.push_str(&format!("- **Path:** `{}`\n", summary.root_path.display()));
    output.push_str(&format!("- **Files analyzed:** {}\n", summary.files_count));
    output.push_str(&format!(
        "- **Directories analyzed:** {}\n",
        summary.directories_count
    ));
    output.push_str(&format!(
        "- **Lines of code:** {}\n\n",
        summary.lines_of_code
    ));

    output.push_str("## Languages\n\n");

    if summary.languages.is_empty() {
        output.push_str("No languages detected.\n\n");
    } else {
        output.push_str("| Language | Files |\n");
        output.push_str("| --- | ---: |\n");

        for language in &summary.languages {
            output.push_str(&format!(
                "| {} | {} |\n",
                escape_table_cell(&language.name),
                language.files_count
            ));
        }

        output.push('\n');
    }

    output.push_str("## Findings\n\n");

    if summary.findings.is_empty() {
        output.push_str("No findings found.\n\n");
    } else {
        output.push_str("| Severity | Rule | Title | Evidence |\n");
        output.push_str("| --- | --- | --- | --- |\n");

        for finding in &summary.findings {
            let evidence = finding
                .evidence
                .first()
                .map(|evidence| {
                    format!(
                        "`{}:{}` — {}",
                        evidence.path.display(),
                        evidence.line_start,
                        evidence.snippet.trim()
                    )
                })
                .unwrap_or_else(|| "No evidence".to_string());

            output.push_str(&format!(
                "| {} | `{}` | {} | {} |\n",
                finding.severity_label(),
                finding.rule_id,
                escape_table_cell(&finding.title),
                escape_table_cell(&evidence)
            ));
        }

        output.push('\n');
    }

    output.push_str("## Markers\n\n");

    let marker_findings: Vec<_> = summary
        .findings
        .iter()
        .filter(|f| f.rule_id.starts_with("code-marker."))
        .collect();

    if marker_findings.is_empty() {
        output.push_str("No TODO/FIXME/HACK markers found.\n");
    } else {
        output.push_str("| Type | File | Line | Snippet |\n");
        output.push_str("| --- | --- | ---: | --- |\n");

        for finding in marker_findings {
            let kind = finding
                .rule_id
                .strip_prefix("code-marker.")
                .unwrap_or("")
                .to_uppercase();

            if let Some(ev) = finding.evidence.first() {
                output.push_str(&format!(
                    "| {} | `{}` | {} | {} |\n",
                    kind,
                    ev.path.display(),
                    ev.line_start,
                    escape_table_cell(ev.snippet.trim())
                ));
            }
        }
    }

    output
}

fn escape_table_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}
