use crate::commands::progress::{finish_spinner, make_spinner};
use crate::commands::scan_config::{ScanConfigOverrides, build_scan_config};
use crate::commands::{CliExit, EXIT_RUNTIME, EXIT_USAGE};
use repopilot::config::loader::{load_default_config, load_optional_config};
use repopilot::config::model::RepoPilotConfig;
use repopilot::config::presets::{Preset, apply_preset};
use repopilot::findings::feedback::apply_local_feedback;
use repopilot::findings::filter::FindingFilter;
use repopilot::findings::visibility::{FindingVisibilityProfile, apply_visibility_profile};
use repopilot::scan::scanner::{scan_changed_with_config, scan_path_with_config};
use repopilot::scan::types::ScanSummary;
use repopilot::scan::workspace_scan::scan_workspace_with_config;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub enum ProductScanMode {
    Full,
    Workspace,
    Changed { since: Option<String> },
}

pub struct ProductScanRequest {
    pub path: PathBuf,
    pub config_path: Option<PathBuf>,
    pub overrides: ScanConfigOverrides,
    pub preset: Option<String>,
    pub mode: ProductScanMode,
    pub ignore_feedback: bool,
    pub visibility_profile: FindingVisibilityProfile,
    pub pre_visibility_filter: FindingFilter,
}

pub struct ProductScanResult {
    pub summary: ScanSummary,
    pub repo_config: RepoPilotConfig,
    pub scan_elapsed: Duration,
}

pub fn run_product_scan(
    request: ProductScanRequest,
) -> Result<ProductScanResult, Box<dyn std::error::Error>> {
    let mut repo_config = match &request.config_path {
        Some(config_path) => load_optional_config(config_path)?,
        None => load_default_config()?,
    };

    if let Some(preset) = request.preset.as_deref() {
        let preset = preset.parse::<Preset>().map_err(|_| CliExit {
            code: EXIT_USAGE,
            message: format!("Invalid preset '{preset}'. Expected: strict, balanced, lenient"),
        })?;
        apply_preset(&mut repo_config, preset);
    }

    let scan_config = build_scan_config(&repo_config, request.overrides);
    let pb = make_spinner("Scanning...");
    let scan_start = Instant::now();
    let scan_result = match &request.mode {
        ProductScanMode::Full => scan_path_with_config(&request.path, &scan_config),
        ProductScanMode::Workspace => scan_workspace_with_config(&request.path, &scan_config),
        ProductScanMode::Changed { since } => {
            scan_changed_with_config(&request.path, &scan_config, since.as_deref())
        }
    };
    let scan_elapsed = scan_start.elapsed();
    finish_spinner(pb);

    let mut summary = scan_result?;
    validate_engine_contract(&mut summary);

    if !request.ignore_feedback {
        apply_local_feedback(&mut summary, &request.path)?;
    }

    if !request.pre_visibility_filter.is_empty() {
        request.pre_visibility_filter.apply_to_summary(&mut summary);
    }

    apply_visibility_profile(&mut summary, request.visibility_profile);

    Ok(ProductScanResult {
        summary,
        repo_config,
        scan_elapsed,
    })
}

pub fn enforce_diagnostics_exit_policy(
    summary: &ScanSummary,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(diagnostic) = summary.first_error_diagnostic() else {
        return Ok(());
    };

    Err(Box::new(CliExit {
        code: EXIT_RUNTIME,
        message: format!(
            "RepoPilot scan completed with reportable error diagnostic `{}`: {}",
            diagnostic.code, diagnostic.message
        ),
    }))
}

pub fn emit_report_only_diagnostics(summary: &ScanSummary) {
    for diagnostic in &summary.diagnostics {
        eprintln!(
            "[{:?}] {}: {}",
            diagnostic.severity, diagnostic.code, diagnostic.message
        );
    }
}

fn validate_engine_contract(summary: &mut ScanSummary) {
    let mut invalid = Vec::new();

    for finding in &summary.findings {
        let mut missing = Vec::new();
        if finding.id.trim().is_empty() {
            missing.push("id");
        }
        if finding.rule_id.trim().is_empty() {
            missing.push("rule_id");
        }
        if finding.recommendation.trim().is_empty() {
            missing.push("recommendation");
        }
        if finding.evidence.is_empty() {
            missing.push("evidence");
        }
        if finding.risk.formula_version.trim().is_empty() {
            missing.push("risk.formula_version");
        }
        if finding.risk.signals.is_empty() {
            missing.push("risk.signals");
        }

        if !missing.is_empty() {
            invalid.push(format!(
                "{} missing {}",
                if finding.rule_id.trim().is_empty() {
                    "<unknown-rule>"
                } else {
                    finding.rule_id.as_str()
                },
                missing.join(", ")
            ));
        }
    }

    if invalid.is_empty() {
        return;
    }

    let sample = invalid
        .iter()
        .take(5)
        .cloned()
        .collect::<Vec<_>>()
        .join("; ");
    summary
        .diagnostics
        .push(repopilot::scan::types::ScanDiagnostic::error(
            "engine.contract-invalid",
            format!(
                "{} finding(s) violated the RepoPilot engine contract: {}",
                invalid.len(),
                sample
            ),
        ));
}
