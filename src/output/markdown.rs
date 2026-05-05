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

    output.push_str("## Code Markers\n\n");

    if summary.markers.is_empty() {
        output.push_str("No TODO/FIXME/HACK markers found.\n");
    } else {
        output.push_str("| Type | File | Line | Evidence |\n");
        output.push_str("| --- | --- | ---: | --- |\n");

        for marker in &summary.markers {
            output.push_str(&format!(
                "| {} | `{}` | {} | {} |\n",
                marker.kind,
                marker.path.display(),
                marker.line_number,
                escape_table_cell(marker.text.trim())
            ));
        }
    }

    output
}

fn escape_table_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}
