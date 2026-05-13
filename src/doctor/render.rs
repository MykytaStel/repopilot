use crate::doctor::model::{DoctorCheck, DoctorReport, DoctorStatus};
use crate::output::OutputFormat;

pub fn render_doctor_report(
    report: &DoctorReport,
    format: OutputFormat,
) -> Result<String, serde_json::Error> {
    match format {
        OutputFormat::Console => Ok(render_console(report)),
        OutputFormat::Markdown => Ok(render_markdown(report)),
        OutputFormat::Json | OutputFormat::Html | OutputFormat::Sarif => {
            serde_json::to_string_pretty(report)
        }
    }
}

fn render_console(report: &DoctorReport) -> String {
    let mut output = String::new();

    output.push_str("RepoPilot Doctor\n\n");
    output.push_str(&format!("Root: {}\n\n", report.root_path));

    output.push_str("Project:\n");
    output.push_str(&format!(
        " Languages: {}\n",
        format_list(&report.project.languages)
    ));
    output.push_str(&format!(
        " Frameworks: {}\n",
        format_list(&report.project.frameworks)
    ));
    output.push_str(&format!(
        " Package managers: {}\n",
        format_list(&report.project.package_managers)
    ));
    output.push_str(&format!(
        " React Native: {}\n\n",
        if report.project.react_native_detected {
            "detected"
        } else {
            "not detected"
        }
    ));

    output.push_str("Audit scope:\n");
    output.push_str(&format!(
        " Files discovered: {}\n",
        report.scan.files_discovered
    ));
    output.push_str(&format!(
        " Files analyzed: {}\n",
        report.scan.files_analyzed
    ));
    output.push_str(&format!(
        " Files skipped (.repopilotignore): {}\n",
        report.scan.files_skipped_repopilotignore
    ));
    output.push_str(&format!(
        " Files skipped (limit): {}\n",
        report.scan.files_skipped_by_limit
    ));
    output.push_str(&format!(
        " Files skipped (low-signal): {}\n",
        report.scan.files_skipped_low_signal
    ));
    output.push_str(&format!(
        " Binary files skipped: {}\n",
        report.scan.binary_files_skipped
    ));
    output.push_str(&format!(
        " Large files skipped: {}\n\n",
        report.scan.large_files_skipped
    ));

    output.push_str("Checks:\n");
    for check in &report.checks {
        output.push_str(&format_check_line(check));
    }

    output.push_str("\nRecommendations:\n");
    for recommendation in &report.recommendations {
        output.push_str(&format!(" - {recommendation}\n"));
    }

    output.push_str("\nSuggested next steps:\n");
    for (index, step) in report.next_steps.iter().enumerate() {
        output.push_str(&format!(" {}. {}\n", index + 1, step.command));
        output.push_str(&format!("    {}\n", step.reason));
    }

    output.push_str("\nRecommended next command:\n");
    output.push_str(&format!(" {}\n", report.next_command));

    output
}

fn render_markdown(report: &DoctorReport) -> String {
    let mut output = String::new();

    output.push_str("# RepoPilot Doctor\n\n");
    output.push_str(&format!("- **Root:** `{}`\n", report.root_path));
    output.push_str(&format!(
        "- **Languages:** {}\n",
        markdown_list(&report.project.languages)
    ));
    output.push_str(&format!(
        "- **Frameworks:** {}\n",
        markdown_list(&report.project.frameworks)
    ));
    output.push_str(&format!(
        "- **Package managers:** {}\n",
        markdown_list(&report.project.package_managers)
    ));
    output.push_str(&format!(
        "- **React Native:** {}\n\n",
        if report.project.react_native_detected {
            "detected"
        } else {
            "not detected"
        }
    ));

    output.push_str("## Audit scope\n\n");
    output.push_str(&format!(
        "- **Files discovered:** {}\n",
        report.scan.files_discovered
    ));
    output.push_str(&format!(
        "- **Files analyzed:** {}\n",
        report.scan.files_analyzed
    ));
    output.push_str(&format!(
        "- **Skipped by `.repopilotignore`:** {}\n",
        report.scan.files_skipped_repopilotignore
    ));
    output.push_str(&format!(
        "- **Skipped by limit:** {}\n",
        report.scan.files_skipped_by_limit
    ));
    output.push_str(&format!(
        "- **Skipped low-signal files:** {}\n",
        report.scan.files_skipped_low_signal
    ));
    output.push_str(&format!(
        "- **Binary files skipped:** {}\n",
        report.scan.binary_files_skipped
    ));
    output.push_str(&format!(
        "- **Large files skipped:** {}\n\n",
        report.scan.large_files_skipped
    ));

    output.push_str("## Checks\n\n");
    for check in &report.checks {
        output.push_str(&format!(
            "- {} **{}** — {}\n  - {}\n",
            check.status.icon(),
            check.title,
            markdown_status(check.status),
            check.detail
        ));
    }

    output.push_str("\n## Recommendations\n\n");
    for recommendation in &report.recommendations {
        output.push_str(&format!("- {recommendation}\n"));
    }

    output.push_str("\n## Suggested next steps\n\n");
    for (index, step) in report.next_steps.iter().enumerate() {
        output.push_str(&format!("{}. `{}`\n", index + 1, step.command));
        output.push_str(&format!("   - {}\n", step.reason));
    }

    output.push_str("\n## Recommended next command\n\n");
    output.push_str("```bash\n");
    output.push_str(&report.next_command);
    output.push('\n');
    output.push_str("```\n");

    output
}

fn format_check_line(check: &DoctorCheck) -> String {
    format!(
        " {} {:<4} {} — {}\n",
        check.status.icon(),
        check.status.label(),
        check.title,
        check.detail
    )
}

fn format_list(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values.join(", ")
    }
}

fn markdown_list(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values
            .iter()
            .map(|value| format!("`{value}`"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn markdown_status(status: DoctorStatus) -> &'static str {
    match status {
        DoctorStatus::Pass => "pass",
        DoctorStatus::Warn => "warning",
        DoctorStatus::Fail => "failed",
    }
}
