use crate::cli::{FailOnArg, OutputFormatArg};
use crate::commands::{CliExit, build_scan_config};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use repopilot::baseline::diff::{all_findings_new, diff_summary_against_baseline};
use repopilot::baseline::gate::evaluate_ci_gate;
use repopilot::baseline::reader::read_baseline;
use repopilot::config::loader::{load_default_config, load_optional_config};
use repopilot::config::presets::{Preset, apply_preset};
use repopilot::findings::types::Severity;
use repopilot::output::{render_baseline_scan_report, render_scan_summary};
use repopilot::report::writer::write_report;
use repopilot::scan::config::ScanConfig;
use repopilot::scan::scanner::scan_path_with_config;
use repopilot::scan::types::{LanguageSummary, ScanSummary};
use repopilot::scan::workspace::{WorkspacePackage, detect_workspace_packages};
use std::collections::BTreeMap;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

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
    verbose: bool,
    preset: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut repo_config = match config {
        Some(config_path) => load_optional_config(&config_path)?,
        None => load_default_config()?,
    };

    if let Some(preset_str) = preset.as_deref() {
        match preset_str.parse::<Preset>() {
            Ok(p) => apply_preset(&mut repo_config, p),
            Err(_) => eprintln!(
                "Warning: unknown preset '{}'. Expected: strict, balanced, lenient",
                preset_str
            ),
        }
    }

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
    let scan_start = Instant::now();

    let mut summary = if workspace {
        scan_workspace(&path, &scan_config)?
    } else {
        scan_path_with_config(&path, &scan_config)?
    };

    let scan_elapsed = scan_start.elapsed();
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
        let render_start = Instant::now();
        let rendered_report =
            render_baseline_scan_report(&baseline_report, output_format, ci_gate.as_ref())?;
        let render_elapsed = render_start.elapsed();

        write_report(&rendered_report, output.as_deref())?;

        if verbose {
            let internal_us = baseline_report.summary.scan_duration_us;
            let total_ms = scan_elapsed.as_millis();
            let render_ms = render_elapsed.as_millis();
            eprintln!(
                "\n[verbose] Scan: {total_ms}ms (engine: {:.0}ms) · Render: {render_ms}ms",
                internal_us as f64 / 1000.0
            );
        }

        if let Some(ci_gate) = ci_gate
            && let Some(message) = ci_gate.failure_message()
        {
            return Err(Box::new(CliExit { code: 1, message }));
        }

        return Ok(());
    }

    let render_start = Instant::now();
    let rendered_report = render_scan_summary(&summary, output_format)?;
    let render_elapsed = render_start.elapsed();

    write_report(&rendered_report, output.as_deref())?;

    if verbose {
        let internal_us = summary.scan_duration_us;
        let total_ms = scan_elapsed.as_millis();
        let render_ms = render_elapsed.as_millis();
        eprintln!(
            "\n[verbose] Scan: {total_ms}ms (engine: {:.0}ms) · Render: {render_ms}ms",
            internal_us as f64 / 1000.0
        );
    }

    Ok(())
}

fn scan_workspace(path: &Path, scan_config: &ScanConfig) -> Result<ScanSummary, std::io::Error> {
    let packages = detect_workspace_packages(path);
    if packages.is_empty() {
        eprintln!(
            "Warning: --workspace specified but no workspace packages found under {}. \
             Falling back to single-package scan.",
            path.display()
        );
        return scan_path_with_config(path, scan_config);
    }

    let root_scan_config = workspace_root_config(scan_config, path, &packages);
    let mut merged = scan_path_with_config(path, &root_scan_config)?;

    // Scan packages in parallel; collect (name, result) pairs then merge sequentially.
    let pkg_results: Vec<(String, Result<_, _>)> = packages
        .par_iter()
        .map(|pkg| {
            (
                pkg.name.clone(),
                scan_path_with_config(&pkg.root, scan_config),
            )
        })
        .collect();

    for (name, result) in pkg_results {
        match result {
            Ok(pkg_summary) => merge_package_summary(&mut merged, pkg_summary, &name),
            Err(err) => eprintln!("Warning: failed to scan workspace package '{name}': {err}"),
        }
    }

    Ok(merged)
}

fn workspace_root_config(
    scan_config: &ScanConfig,
    root: &Path,
    packages: &[WorkspacePackage],
) -> ScanConfig {
    let mut config = scan_config.clone();
    for package in packages {
        if let Some(relative_path) = workspace_relative_path(root, &package.root) {
            config.ignored_paths.push(relative_path);
        }
    }
    config
}

fn workspace_relative_path(root: &Path, package_root: &Path) -> Option<String> {
    package_root
        .strip_prefix(root)
        .ok()
        .and_then(|path| path.to_str())
        .filter(|path| !path.is_empty())
        .map(|path| path.replace('\\', "/"))
}

fn merge_package_summary(merged: &mut ScanSummary, mut package: ScanSummary, package_name: &str) {
    for finding in &mut package.findings {
        finding.workspace_package = Some(package_name.to_string());
    }

    merged.files_count += package.files_count;
    merged.directories_count += package.directories_count;
    merged.lines_of_code += package.lines_of_code;
    merged.skipped_files_count += package.skipped_files_count;
    merged.skipped_bytes = merged.skipped_bytes.saturating_add(package.skipped_bytes);
    merged.scan_duration_us = merged
        .scan_duration_us
        .saturating_add(package.scan_duration_us);
    merge_language_summaries(&mut merged.languages, package.languages);
    merged.findings.extend(package.findings);
}

fn merge_language_summaries(target: &mut Vec<LanguageSummary>, source: Vec<LanguageSummary>) {
    let mut counts: BTreeMap<String, usize> = target
        .drain(..)
        .map(|language| (language.name, language.files_count))
        .collect();

    for language in source {
        *counts.entry(language.name).or_insert(0) += language.files_count;
    }

    let mut merged: Vec<_> = counts
        .into_iter()
        .map(|(name, files_count)| LanguageSummary { name, files_count })
        .collect();
    merged.sort_by(|left, right| {
        right
            .files_count
            .cmp(&left.files_count)
            .then_with(|| left.name.cmp(&right.name))
    });

    *target = merged;
}

pub(crate) fn make_spinner() -> Option<ProgressBar> {
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

pub(crate) fn finish_spinner(pb: Option<ProgressBar>) {
    if let Some(pb) = pb {
        pb.finish_and_clear();
    }
}
