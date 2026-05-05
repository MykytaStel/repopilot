use crate::scan::types::ScanSummary;

pub fn render(summary: &ScanSummary) -> String {
    let mut output = String::new();

    output.push_str("RepoPilot Scan\n");
    output.push_str(&format!("Path: {}\n\n", summary.root_path.display()));

    output.push_str(&format!("Files analyzed: {}\n", summary.files_count));
    output.push_str(&format!(
        "Directories analyzed: {}\n",
        summary.directories_count
    ));
    output.push_str(&format!("Lines of code: {}\n\n", summary.lines_of_code));

    output.push_str("Languages:\n");

    if summary.languages.is_empty() {
        output.push_str("  No languages detected\n");
    } else {
        for language in &summary.languages {
            output.push_str(&format!(
                "  {}: {} files\n",
                language.name, language.files_count
            ));
        }
    }

    output.push_str("\nFindings:\n");

    if summary.findings.is_empty() {
        output.push_str("  No findings found\n");
    } else {
        for finding in &summary.findings {
            output.push_str(&format!(
                "  [{}] {} — {}\n",
                finding.severity_label(),
                finding.rule_id,
                finding.title
            ));

            for evidence in &finding.evidence {
                output.push_str(&format!(
                    "    Evidence: {}:{} — {}\n",
                    evidence.path.display(),
                    evidence.line_start,
                    evidence.snippet.trim()
                ));
            }
        }
    }

    output
}
