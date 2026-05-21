pub(crate) fn render_languages_section(output: &mut String, summary: &ScanSummary) {
    output.push_str("Languages:\n");

    if summary.languages.is_empty() {
        output.push_str("  No languages detected\n\n");
        return;
    }

    for language in &summary.languages {
        writeln!(
            output,
            "  {}: {} files",
            language.name, language.files_analyzed
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
    writeln!(output, "Frameworks: {}\n", labels.join(" | ")).unwrap();
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

    output.push_str("Framework projects:\n");
    for project in nested_projects {
        let labels: Vec<String> = project.frameworks.iter().map(|f| f.label()).collect();
        writeln!(
            output,
            "  {}: {}",
            project.path.display(),
            labels.join(" | ")
        )
        .unwrap();
    }
    output.push('\n');
}

fn render_scan_input(output: &mut String, summary: &ScanSummary) {
    output.push_str("Scan input:\n");

    if let Some(path) = &summary.repopilotignore_path {
        writeln!(output, " .repopilotignore: {}", path.display()).unwrap();
    }

    if summary.files_skipped_repopilotignore > 0 {
        writeln!(
            output,
            " Files skipped (.repopilotignore): {:>7}",
            summary.files_skipped_repopilotignore
        )
        .unwrap();
    }

    writeln!(output, " Files discovered: {:>7}", summary.files_discovered).unwrap();

    if summary.files_skipped_by_limit > 0 {
        writeln!(
            output,
            " Files skipped (limit): {:>7}",
            summary.files_skipped_by_limit
        )
        .unwrap();
    }

    writeln!(output, " Files analyzed: {:>7}", summary.files_analyzed).unwrap();

    if summary.large_files_skipped > 0 {
        writeln!(
            output,
            " Large files skipped: {:>7}",
            summary.large_files_skipped
        )
        .unwrap();
    }

    if summary.binary_files_skipped > 0 {
        writeln!(
            output,
            " Binary files skipped: {:>7}",
            summary.binary_files_skipped
        )
        .unwrap();
    }

    if summary.files_skipped_low_signal > 0 {
        writeln!(
            output,
            " Low-signal files skipped:{:>7}",
            summary.files_skipped_low_signal
        )
        .unwrap();
    }
}

fn health_score_bar(score: u8) -> &'static str {
    match score {
        90..=100 => "[##########] Excellent",
        75..=89 => "[########  ] Good",
        60..=74 => "[######    ] Fair",
        40..=59 => "[####      ] Poor",
        _ => "[##        ] Critical",
    }
}
