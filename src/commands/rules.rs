use crate::cli::{CompareOutputFormatArg, RuleLifecycleArg, SignalSourceArg};
use crate::commands::{CliExit, EXIT_RUNTIME, EXIT_USAGE};
use repopilot::output::OutputFormat;
use repopilot::report::writer::write_report;
use repopilot::rules::catalog::{RuleCatalogFilter, inspect_rule as build_rule_report};
use repopilot::rules::{RuleLifecycle, SignalSource};
use std::path::PathBuf;

pub fn list_rules(
    format: CompareOutputFormatArg,
    lifecycle: Option<RuleLifecycleArg>,
    source: Option<SignalSourceArg>,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = validate_output_format(OutputFormat::from(format))?;
    let report = repopilot::rules::catalog::list_rule_catalog(RuleCatalogFilter {
        lifecycle: lifecycle.map(RuleLifecycle::from),
        source: source.map(SignalSource::from),
    });
    let rendered = repopilot::output::rules::render_catalog(&report, format)?;
    write_report(&rendered, output.as_deref())?;
    Ok(())
}

pub fn inspect_rule(
    rule_id: String,
    format: CompareOutputFormatArg,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(report) = build_rule_report(&rule_id) else {
        return Err(Box::new(CliExit {
            code: EXIT_USAGE,
            message: format!("Unknown RepoPilot rule `{rule_id}`"),
        }));
    };

    let format = validate_output_format(OutputFormat::from(format))?;
    let rendered = repopilot::output::rules::render_rule(&report, format)?;
    write_report(&rendered, output.as_deref())?;
    Ok(())
}

pub fn eval_rules(
    rule: Option<String>,
    fixtures: Option<PathBuf>,
    format: CompareOutputFormatArg,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(rule_id) = &rule
        && repopilot::rules::lookup_rule_metadata(rule_id).is_none()
    {
        return Err(Box::new(CliExit {
            code: EXIT_USAGE,
            message: format!("Unknown RepoPilot rule `{rule_id}`"),
        }));
    }

    let format = validate_output_format(OutputFormat::from(format))?;
    let report = repopilot::rules::eval::fixtures::evaluate_rule_fixtures(
        rule.as_deref(),
        fixtures.as_deref(),
    )
    .map_err(|error| CliExit {
        code: EXIT_RUNTIME,
        message: error.to_string(),
    })?;
    let rendered = repopilot::output::rules::render_eval_report(&report, format)?;
    write_report(&rendered, output.as_deref())?;
    Ok(())
}

fn validate_output_format(format: OutputFormat) -> Result<OutputFormat, CliExit> {
    match format {
        OutputFormat::Console | OutputFormat::Json | OutputFormat::Markdown => Ok(format),
        OutputFormat::Html | OutputFormat::Sarif => Err(CliExit {
            code: EXIT_USAGE,
            message: "`inspect rules` supports only console, markdown, and json output".to_string(),
        }),
    }
}
