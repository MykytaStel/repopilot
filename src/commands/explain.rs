use crate::cli::{CompareOutputFormatArg, SeverityArg};
use crate::commands::severity_arg_into;
use repopilot::explain::{build_explain_report, render_explain_report};
use repopilot::output::OutputFormat;
use repopilot::report::writer::write_report;
use std::path::PathBuf;

pub fn run(
    path: PathBuf,
    rule: Option<String>,
    signal: Option<String>,
    severity: SeverityArg,
    format: CompareOutputFormatArg,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let report = build_explain_report(
        &path,
        rule.as_deref(),
        signal.as_deref(),
        severity_arg_into(severity),
    )?;
    let rendered = render_explain_report(&report, OutputFormat::from(format))?;

    write_report(&rendered, output.as_deref())?;

    Ok(())
}
