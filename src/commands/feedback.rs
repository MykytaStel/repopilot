use crate::cli::CompareOutputFormatArg;
use crate::commands::{CliExit, EXIT_USAGE};
use repopilot::config::loader::load_default_config;
use repopilot::findings::feedback::{
    LocalFeedbackReport, LocalFeedbackValidation, LocalSuppression, apply_local_feedback,
    validate_local_feedback,
};
use repopilot::output::OutputFormat;
use repopilot::report::writer::write_report;
use repopilot::scan::scanner::scan_path_with_config;
use repopilot::scan::types::ScanDiagnostic;
use serde::Serialize;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
struct FeedbackInspection {
    feedback_path: PathBuf,
    exists: bool,
    suppressions_loaded: usize,
    invalid_suppressions_count: usize,
    parse_error: Option<String>,
    diagnostics: Vec<ScanDiagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    evaluation: Option<FeedbackEvaluation>,
}

#[derive(Debug, Serialize)]
struct FeedbackEvaluation {
    findings_after_feedback: usize,
    suppressed_findings_count: usize,
    unmatched_suppressions_count: usize,
    unmatched_suppressions: Vec<LocalSuppression>,
}

pub fn run(
    path: PathBuf,
    format: CompareOutputFormatArg,
    output: Option<PathBuf>,
    evaluate: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let validation = validate_local_feedback(&path)?;
    let mut inspection = FeedbackInspection::from_validation(validation);

    if evaluate {
        inspection.evaluate(&path)?;
    }

    let rendered = render_feedback_inspection(&inspection, OutputFormat::from(format))?;
    write_report(&rendered, output.as_deref())?;
    Ok(())
}

impl FeedbackInspection {
    fn from_validation(validation: LocalFeedbackValidation) -> Self {
        Self {
            feedback_path: validation.feedback_path,
            exists: validation.exists,
            suppressions_loaded: validation.suppressions.len(),
            invalid_suppressions_count: validation.invalid_suppressions_count,
            parse_error: validation.parse_error,
            diagnostics: validation.diagnostics,
            evaluation: None,
        }
    }

    fn evaluate(&mut self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let repo_config = load_default_config()?;
        let scan_config = repo_config.to_scan_config();
        let mut summary = scan_path_with_config(path, &scan_config)?;
        let report = apply_local_feedback(&mut summary, path)?;

        self.diagnostics = summary.diagnostics;
        self.evaluation = Some(FeedbackEvaluation::from_report(
            summary.findings.len(),
            report,
        ));
        Ok(())
    }
}

impl FeedbackEvaluation {
    fn from_report(findings_after_feedback: usize, report: LocalFeedbackReport) -> Self {
        Self {
            findings_after_feedback,
            suppressed_findings_count: report.suppressed_findings_count,
            unmatched_suppressions_count: report.unmatched_suppressions_count,
            unmatched_suppressions: report.unmatched_suppressions,
        }
    }
}

fn render_feedback_inspection(
    inspection: &FeedbackInspection,
    format: OutputFormat,
) -> Result<String, Box<dyn std::error::Error>> {
    match validate_feedback_output_format(format)? {
        OutputFormat::Console => Ok(render_console(inspection)),
        OutputFormat::Markdown => Ok(render_markdown(inspection)),
        OutputFormat::Json => Ok(serde_json::to_string_pretty(inspection)?),
        OutputFormat::Html | OutputFormat::Sarif => unreachable!("validated output format"),
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
    output.push_str(&format!(
        "Suppressions loaded: {}\nInvalid suppressions: {}\n",
        inspection.suppressions_loaded, inspection.invalid_suppressions_count
    ));
    if let Some(error) = &inspection.parse_error {
        output.push_str(&format!("Parse error: {error}\n"));
    }

    if let Some(evaluation) = &inspection.evaluation {
        output.push_str(&format!(
            "Findings after feedback: {}\nSuppressed findings: {}\nUnmatched suppressions: {}\n",
            evaluation.findings_after_feedback,
            evaluation.suppressed_findings_count,
            evaluation.unmatched_suppressions_count
        ));
        render_unmatched_suppressions_console(&mut output, &evaluation.unmatched_suppressions);
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
    output.push_str(&format!(
        "- **Suppressions loaded:** {}\n- **Invalid suppressions:** {}\n",
        inspection.suppressions_loaded, inspection.invalid_suppressions_count
    ));
    if let Some(error) = &inspection.parse_error {
        output.push_str(&format!("- **Parse error:** `{error}`\n"));
    }

    if let Some(evaluation) = &inspection.evaluation {
        output.push_str(&format!(
            "- **Findings after feedback:** {}\n- **Suppressed findings:** {}\n- **Unmatched suppressions:** {}\n",
            evaluation.findings_after_feedback,
            evaluation.suppressed_findings_count,
            evaluation.unmatched_suppressions_count
        ));
        render_unmatched_suppressions_markdown(&mut output, &evaluation.unmatched_suppressions);
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

fn render_unmatched_suppressions_console(output: &mut String, suppressions: &[LocalSuppression]) {
    if suppressions.is_empty() {
        return;
    }

    output.push_str("Unmatched suppression entries:\n");
    for suppression in suppressions {
        output.push_str(&format!(
            "  #{} {} {}\n",
            suppression.index, suppression.rule_id, suppression.path
        ));
    }
}

fn render_unmatched_suppressions_markdown(output: &mut String, suppressions: &[LocalSuppression]) {
    if suppressions.is_empty() {
        return;
    }

    output.push_str("\n## Unmatched Suppressions\n\n");
    for suppression in suppressions {
        output.push_str(&format!(
            "- `#{}` `{}` `{}`\n",
            suppression.index, suppression.rule_id, suppression.path
        ));
    }
}

fn validate_feedback_output_format(format: OutputFormat) -> Result<OutputFormat, CliExit> {
    match format {
        OutputFormat::Console | OutputFormat::Json | OutputFormat::Markdown => Ok(format),
        OutputFormat::Html | OutputFormat::Sarif => Err(CliExit {
            code: EXIT_USAGE,
            message: format!(
                "`inspect feedback` supports only console, markdown, and json output; received {}",
                output_format_name(format)
            ),
        }),
    }
}

fn output_format_name(format: OutputFormat) -> &'static str {
    match format {
        OutputFormat::Console => "console",
        OutputFormat::Html => "html",
        OutputFormat::Json => "json",
        OutputFormat::Markdown => "markdown",
        OutputFormat::Sarif => "sarif",
    }
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}
