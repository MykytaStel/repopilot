use crate::cli::{BaselineCommands, ScanProfileArg};
use crate::commands::filters::{priority_arg_into, severity_arg_into};
use crate::commands::product_scan::{ProductScanMode, ProductScanRequest, run_product_scan};
use crate::commands::scan_config::ScanConfigOverrides;
use chrono::{SecondsFormat, Utc};
use repopilot::baseline::key::normalized_relative_path;
use repopilot::baseline::model::Baseline;
use repopilot::baseline::writer::{BaselineWriteError, write_baseline};
use repopilot::findings::filter::FindingFilter;
use repopilot::findings::types::Severity;
use repopilot::findings::visibility::FindingVisibilityProfile;
use repopilot::scan::types::ScanSummary;
use std::path::{Path, PathBuf};

pub fn run(command: BaselineCommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        BaselineCommands::Create(options) => {
            let path = options.path;
            let output = options.output;
            let force = options.force;
            let config = options.config;
            let ignore_feedback = options.ignore_feedback;
            let output_path = output.unwrap_or_else(|| default_baseline_path(&path));
            let visibility_profile = baseline_visibility_profile(options.profile);
            let pre_visibility_filter = FindingFilter {
                min_severity: options.min_severity.map(severity_arg_into),
                ..FindingFilter::default()
            };
            let priority_filter = FindingFilter {
                min_priority: options.min_priority.map(priority_arg_into),
                ..FindingFilter::default()
            };

            let scan_result = run_product_scan(ProductScanRequest {
                path: path.clone(),
                config_path: config,
                overrides: ScanConfigOverrides::default(),
                preset: None,
                mode: ProductScanMode::Full,
                no_progress: false,
                ignore_feedback,
                visibility_profile,
                pre_visibility_filter,
            })?;
            let mut summary = scan_result.summary;
            // Priority is derived from the scan result, so — like `scan`
            // itself — it filters after the scan runs rather than as a
            // pre-visibility filter. Applied before the baseline is built, so
            // a finding this filter would hide is never stored as accepted.
            priority_filter.apply_to_summary(&mut summary);
            let baseline = Baseline::from_scan_summary(
                &summary,
                &summary.root_path,
                display_baseline_root(&path),
                current_timestamp(),
            );

            match write_baseline(&baseline, &output_path, force) {
                Ok(()) => {
                    print_baseline_create_summary(&path, &output_path, &summary);
                    Ok(())
                }
                Err(BaselineWriteError::AlreadyExists { .. }) => Err(format!(
                    "Baseline already exists at {}. Use `repopilot baseline create {} --force` to overwrite it.",
                    output_path.display(),
                    path.display()
                )
                .into()),
                Err(error) => Err(Box::new(error)),
            }
        }
    }
}

fn baseline_visibility_profile(profile: Option<ScanProfileArg>) -> FindingVisibilityProfile {
    match profile {
        Some(ScanProfileArg::Strict) => FindingVisibilityProfile::Strict,
        Some(ScanProfileArg::Default) | None => FindingVisibilityProfile::Default,
    }
}

fn default_baseline_path(scan_path: &Path) -> PathBuf {
    if scan_path == Path::new(".") {
        return PathBuf::from(".repopilot/baseline.json");
    }

    if scan_path.is_file() {
        return scan_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(".repopilot/baseline.json");
    }

    scan_path.join(".repopilot/baseline.json")
}

fn display_baseline_root(path: &Path) -> String {
    if path.is_absolute()
        && let Ok(current_dir) = std::env::current_dir()
    {
        return normalized_relative_path(path, &current_dir);
    }

    normalized_relative_path(path, Path::new("."))
}

fn current_timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn print_baseline_create_summary(scan_path: &Path, output_path: &Path, summary: &ScanSummary) {
    let counts = SeverityCounts::from_summary(summary);

    println!("RepoPilot Baseline");
    println!();
    println!("Scanned path: {}", scan_path.display());
    println!("Baseline written to: {}", output_path.display());
    println!();
    println!("Stored findings:");
    println!("- Critical: {}", counts.critical);
    println!("- High: {}", counts.high);
    println!("- Medium: {}", counts.medium);
    println!("- Low: {}", counts.low);
    println!("- Info: {}", counts.info);
}

#[derive(Default)]
struct SeverityCounts {
    critical: usize,
    high: usize,
    medium: usize,
    low: usize,
    info: usize,
}

impl SeverityCounts {
    fn from_summary(summary: &ScanSummary) -> Self {
        let mut counts = SeverityCounts::default();

        for finding in &summary.artifacts.findings {
            match finding.severity {
                Severity::Critical => counts.critical += 1,
                Severity::High => counts.high += 1,
                Severity::Medium => counts.medium += 1,
                Severity::Low => counts.low += 1,
                Severity::Info => counts.info += 1,
            }
        }

        counts
    }
}
