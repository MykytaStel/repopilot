use crate::cli::CompareOutputFormatArg;
use repopilot::config::loader::load_default_config;
use repopilot::findings::feedback::{
    LocalFeedbackReport, apply_local_feedback, validate_local_feedback,
};
use repopilot::output::OutputFormat;
use repopilot::report::writer::write_report;
use repopilot::scan::scanner::scan_path_with_config;
use repopilot::scan::types::ScanDiagnostic;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
struct FeedbackInspection {
    feedback_path: PathBuf,
    exists: bool,
    findings_after_feedback: usize,
    local_feedback: Option<LocalFeedbackReport>,
    diagnostics: Vec<ScanDiagnostic>,
}

pub fn run(
    path: PathBuf,
    format: CompareOutputFormatArg,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let validation = validate_local_feedback(&path)?;
    let inspection = if validation.exists {
        let repo_config = load_default_config()?;
        let scan_config = repo_config.to_scan_config();
        let mut summary = scan_path_with_config(&path, &scan_config)?;
        apply_local_feedback(&mut summary, &path)?;

        FeedbackInspection {
            feedback_path: validation.feedback_path,
            exists: true,
            findings_after_feedback: summary.findings.len(),
            local_feedback: summary.local_feedback,
            diagnostics: summary.diagnostics,
        }
    } else {
        FeedbackInspection {
            feedback_path: validation.feedback_path,
            exists: false,
            findings_after_feedback: 0,
            local_feedback: None,
            diagnostics: validation.diagnostics,
        }
    };

    let rendered = render_feedback_inspection(&inspection, OutputFormat::from(format))?;
    write_report(&rendered, output.as_deref())?;
    Ok(())
}

fn render_feedback_inspection(
    inspection: &FeedbackInspection,
    format: OutputFormat,
) -> Result<String, serde_json::Error> {
    match format {
        OutputFormat::Console => Ok(render_console(inspection)),
        OutputFormat::Markdown => Ok(render_markdown(inspection)),
        OutputFormat::Json | OutputFormat::Html | OutputFormat::Sarif => {
            serde_json::to_string_pretty(inspection)
        }
    }
}

fn render_console(inspection: &FeedbackInspection) -> String {
    let mut output = String::new();
    output.push_str("RepoPilot Feedback Diagnostics\n\n");
    output.push_str(&format!(
        "Feedback file: {}\n",
        inspection.feedback_path.display()
    ));
    output.push_str(&format!("Exists: {}\n", yes_no(inspection.exists)));

    match &inspection.local_feedback {
        Some(feedback) => {
            output.push_str(&format!(
                "Suppressions loaded: {}\nSuppressed findings: {}\nUnmatched suppressions: {}\nInvalid suppressions: {}\nFindings after feedback: {}\n",
                feedback.suppressions_loaded,
                feedback.suppressed_findings_count,
                feedback.unmatched_suppressions_count,
                feedback.invalid_suppressions_count,
                inspection.findings_after_feedback
            ));
            if let Some(error) = &feedback.parse_error {
                output.push_str(&format!("Parse error: {error}\n"));
            }
        }
        None => output.push_str("Suppressions loaded: 0\n"),
    }

    render_diagnostics_console(&mut output, &inspection.diagnostics);
    output
}

fn render_markdown(inspection: &FeedbackInspection) -> String {
    let mut output = String::new();
    output.push_str("# RepoPilot Feedback Diagnostics\n\n");
    output.push_str(&format!(
        "- **Feedback file:** `{}`\n",
        inspection.feedback_path.display()
    ));
    output.push_str(&format!("- **Exists:** {}\n", yes_no(inspection.exists)));

    match &inspection.local_feedback {
        Some(feedback) => {
            output.push_str(&format!(
                "- **Suppressions loaded:** {}\n- **Suppressed findings:** {}\n- **Unmatched suppressions:** {}\n- **Invalid suppressions:** {}\n- **Findings after feedback:** {}\n",
                feedback.suppressions_loaded,
                feedback.suppressed_findings_count,
                feedback.unmatched_suppressions_count,
                feedback.invalid_suppressions_count,
                inspection.findings_after_feedback
            ));
            if let Some(error) = &feedback.parse_error {
                output.push_str(&format!("- **Parse error:** `{error}`\n"));
            }
        }
        None => output.push_str("- **Suppressions loaded:** 0\n"),
    }

    render_diagnostics_markdown(&mut output, &inspection.diagnostics);
    output
}

fn render_diagnostics_console(output: &mut String, diagnostics: &[ScanDiagnostic]) {
    if diagnostics.is_empty() {
        return;
    }

    output.push_str("\nDiagnostics:\n");
    for diagnostic in diagnostics {
        let path = diagnostic
            .path
            .as_ref()
            .map(|path| format!(" ({})", path.display()))
            .unwrap_or_default();
        output.push_str(&format!(
            "  [{:?}] {}{}: {}\n",
            diagnostic.severity, diagnostic.code, path, diagnostic.message
        ));
    }
}

fn render_diagnostics_markdown(output: &mut String, diagnostics: &[ScanDiagnostic]) {
    if diagnostics.is_empty() {
        return;
    }

    output.push_str("\n## Diagnostics\n\n");
    for diagnostic in diagnostics {
        let path = diagnostic
            .path
            .as_ref()
            .map(|path| format!(" `{}`", path.display()))
            .unwrap_or_default();
        output.push_str(&format!(
            "- `{:?}` `{}`{}: {}\n",
            diagnostic.severity, diagnostic.code, path, diagnostic.message
        ));
    }
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}
