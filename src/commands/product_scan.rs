use crate::commands::progress::{finish_spinner, make_spinner};
use crate::commands::scan_config::{ScanConfigOverrides, build_scan_config};
use crate::commands::{CliExit, EXIT_RUNTIME, EXIT_USAGE};
use repopilot::config::loader::{load_default_config, load_optional_config};
use repopilot::config::model::RepoPilotConfig;
use repopilot::config::presets::{Preset, apply_preset};
use repopilot::facts::RepoFactsSummary;
use repopilot::findings::feedback::apply_local_feedback;
use repopilot::findings::filter::FindingFilter;
use repopilot::findings::visibility::{FindingVisibilityProfile, apply_visibility_profile};
use repopilot::review::diff::ChangedFile;
use repopilot::scan::scanner::{
    scan_changed_with_config, scan_path_with_config, scan_path_with_config_and_facts_summary,
    scan_resolved_changed_with_config,
};
use repopilot::scan::types::ScanSummary;
use repopilot::scan::workspace_scan::scan_workspace_with_config;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub enum ProductScanMode {
    Full,
    Workspace,
    Changed {
        since: Option<String>,
    },
    ResolvedChanged {
        repo_root: PathBuf,
        changed_files: Vec<ChangedFile>,
        base_ref: Option<String>,
    },
}

pub struct ProductScanRequest {
    pub path: PathBuf,
    pub config_path: Option<PathBuf>,
    pub overrides: ScanConfigOverrides,
    pub preset: Option<String>,
    pub mode: ProductScanMode,
    pub no_progress: bool,
    pub ignore_feedback: bool,
    pub visibility_profile: FindingVisibilityProfile,
    pub pre_visibility_filter: FindingFilter,
}

pub struct ProductScanResult {
    pub summary: ScanSummary,
    pub repo_facts_summary: Option<RepoFactsSummary>,
    pub repo_config: RepoPilotConfig,
    pub scan_elapsed: Duration,
}

pub fn run_product_scan(
    request: ProductScanRequest,
) -> Result<ProductScanResult, Box<dyn std::error::Error>> {
    run_product_scan_internal(request, false)
}

pub fn run_product_scan_with_facts_summary(
    request: ProductScanRequest,
) -> Result<ProductScanResult, Box<dyn std::error::Error>> {
    run_product_scan_internal(request, true)
}

fn run_product_scan_internal(
    request: ProductScanRequest,
    include_repo_facts_summary: bool,
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
    let pb = if request.no_progress {
        None
    } else {
        make_spinner("Scanning...")
    };
    let scan_start = Instant::now();
    let scan_result = match &request.mode {
        ProductScanMode::Full if include_repo_facts_summary => {
            scan_path_with_config_and_facts_summary(&request.path, &scan_config)
                .map(|(summary, facts_summary)| (summary, Some(facts_summary)))
        }
        ProductScanMode::Full => {
            scan_path_with_config(&request.path, &scan_config).map(|summary| (summary, None))
        }
        ProductScanMode::Workspace => {
            scan_workspace_with_config(&request.path, &scan_config).map(|summary| (summary, None))
        }
        ProductScanMode::Changed { since } => {
            scan_changed_with_config(&request.path, &scan_config, since.as_deref())
                .map(|summary| (summary, None))
        }
        ProductScanMode::ResolvedChanged {
            repo_root,
            changed_files,
            base_ref,
        } => scan_resolved_changed_with_config(
            &request.path,
            &scan_config,
            repo_root.clone(),
            changed_files.clone(),
            base_ref.as_deref(),
        )
        .map(|summary| (summary, None)),
    };
    let scan_elapsed = scan_start.elapsed();
    finish_spinner(pb);

    let (mut summary, repo_facts_summary) = scan_result?;

    if !request.ignore_feedback {
        apply_local_feedback(&mut summary, &request.path)?;
    }

    if !request.pre_visibility_filter.is_empty() {
        request.pre_visibility_filter.apply_to_summary(&mut summary);
    }

    apply_visibility_profile(&mut summary, request.visibility_profile);

    Ok(ProductScanResult {
        summary,
        repo_facts_summary,
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
    for diagnostic in &summary.artifacts.diagnostics {
        eprintln!(
            "[{:?}] {}: {}",
            diagnostic.severity, diagnostic.code, diagnostic.message
        );
    }
}
