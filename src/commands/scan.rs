use crate::cli::{FailOnArg, OutputFormatArg};
use crate::commands::{CliExit, build_scan_config};
use indicatif::{ProgressBar, ProgressStyle};
use repopilot::baseline::diff::{all_findings_new, diff_summary_against_baseline};
use repopilot::baseline::gate::evaluate_ci_gate;
use repopilot::baseline::reader::read_baseline;
use repopilot::config::loader::{load_default_config, load_optional_config};
use repopilot::findings::types::Severity;
use repopilot::output::{render_baseline_scan_report, render_scan_summary};
use repopilot::report::writer::write_report;
use repopilot::scan::scanner::scan_path_with_config;
use repopilot::scan::workspace::detect_workspace_packages;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::time::Duration;

#[allow(clippy::too_many_arguments)]
pub fn run(
    path: PathBuf,
    format: Option<OutputFormatArg>,
    output: Option<PathBuf>,
    config: Option<PathBuf>,
    baseline: Option<PathBuf>,
    fail_on: Option<FailOnArg>,
    max_file_loc: Option<usize>,
    max_directory_modules: Option<usize>,
    max_directory_depth: Option<usize>,
    workspace: bool,
    min_severity: Option<Severity>,
) -> Result<(), Box<dyn std::error::Error>> {
    let repo_config = match config {
        Some(config_path) => load_optional_config(&config_path)?,
        None => load_default_config()?,
    };
    let scan_config = build_scan_config(
        &repo_config,
        max_file_loc,
        max_directory_modules,
        max_directory_depth,
    );
    let output_format = format
        .map(Into::into)
        .unwrap_or(repo_config.output.default_format);

    let pb = make_spinner();

    let mut summary = if workspace {
        let packages = detect_workspace_packages(&path);
        if packages.is_empty() {
            eprintln!(
                "Warning: --workspace specified but no workspace packages found under {}. \
                 Falling back to single-package scan.",
                path.display()
            );
            scan_path_with_config(&path, &scan_config)?
        } else {
            let mut merged = scan_path_with_config(&path, &scan_config)?;
            for pkg in &packages {
                match scan_path_with_config(&pkg.root, &scan_config) {
                    Ok(mut pkg_summary) => {
                        for finding in &mut pkg_summary.findings {
                            finding.workspace_package = Some(pkg.name.clone());
                        }
                        merged.findings.extend(pkg_summary.findings);
                        merged.files_count += pkg_summary.files_count;
                        merged.directories_count += pkg_summary.directories_count;
                        merged.lines_of_code += pkg_summary.lines_of_code;
                    }
                    Err(err) => eprintln!(
                        "Warning: failed to scan workspace package '{}': {err}",
                        pkg.name
                    ),
                }
            }
            merged
        }
    } else {
        scan_path_with_config(&path, &scan_config)?
    };

    finish_spinner(pb);

    if let Some(min) = min_severity {
        summary.findings.retain(|f| f.severity >= min);
    }

    if baseline.is_some() || fail_on.is_some() {
        let baseline_report = match baseline {
            Some(baseline_path) => {
                let baseline_file = read_baseline(&baseline_path)?;
                diff_summary_against_baseline(summary, &baseline_file, baseline_path)
            }
            None => all_findings_new(summary),
        };

        let ci_gate = fail_on
            .map(Into::into)
            .map(|fail_on| evaluate_ci_gate(&baseline_report, fail_on));
        let rendered_report =
            render_baseline_scan_report(&baseline_report, output_format, ci_gate.as_ref())?;

        write_report(&rendered_report, output.as_deref())?;

        if let Some(ci_gate) = ci_gate
            && let Some(message) = ci_gate.failure_message()
        {
            return Err(Box::new(CliExit { code: 1, message }));
        }

        return Ok(());
    }

    let rendered_report = render_scan_summary(&summary, output_format)?;
    write_report(&rendered_report, output.as_deref())?;

    Ok(())
}

fn make_spinner() -> Option<ProgressBar> {
    if !std::io::stderr().is_terminal() {
        return None;
    }
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.set_message("Scanning...");
    pb.enable_steady_tick(Duration::from_millis(80));
    Some(pb)
}

fn finish_spinner(pb: Option<ProgressBar>) {
    if let Some(pb) = pb {
        pb.finish_and_clear();
    }
}
