pub(crate) fn render_languages_section(output: &mut String, summary: &ScanSummary) {
    output.push_str("## Languages\n\n");

    if summary.metrics.languages.is_empty() {
        output.push_str("No languages detected.\n\n");
        return;
    }

    output.push_str("| Language | Files |\n");
    output.push_str("| --- | ---: |\n");
    for language in &summary.metrics.languages {
        writeln!(
            output,
            "| {} | {} |",
            escape_table_cell(&language.name),
            language.files_analyzed
        )
        .unwrap();
    }
    output.push('\n');
}

pub(crate) fn render_frameworks_section(output: &mut String, frameworks: &[DetectedFramework]) {
    if frameworks.is_empty() {
        return;
    }
    let labels: Vec<String> = frameworks.iter().map(|f| f.label()).collect();
    output.push_str("## Frameworks\n\n");
    writeln!(output, "{}\n", labels.join(" | ")).unwrap();
}

pub(crate) fn render_framework_projects_section(
    output: &mut String,
    projects: &[FrameworkProject],
) {
    let nested_projects: Vec<_> = projects
        .iter()
        .filter(|project| project.path.as_path() != std::path::Path::new("."))
        .collect();
    if nested_projects.is_empty() {
        return;
    }

    output.push_str("## Framework Projects\n\n");
    output.push_str("| Path | Frameworks |\n");
    output.push_str("| --- | --- |\n");
    for project in nested_projects {
        let labels: Vec<String> = project.frameworks.iter().map(|f| f.label()).collect();
        writeln!(
            output,
            "| `{}` | {} |",
            project.path.display(),
            escape_table_cell(&labels.join(", "))
        )
        .unwrap();
    }
    output.push('\n');
}
