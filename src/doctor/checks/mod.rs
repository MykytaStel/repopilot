pub mod baseline;
pub mod ci;
pub mod config;
pub mod git;
pub mod helpers;
pub mod next_steps;
pub mod scope;

use crate::baseline::reader::read_baseline;
use crate::doctor::model::{DoctorProject, DoctorReport, DoctorScanScope};
use crate::scan::config::ScanConfig;
use crate::scan::scanner::scan_path_with_config;
use std::io;
use std::path::Path;

use baseline::{check_baseline, check_baseline_readable};
use ci::{check_ci_config, check_repopilot_ci, detect_repopilot_ci_config, has_ci_config};
use config::{check_config, check_config_readable, check_repopilotignore, config_file_is_readable};
use git::check_git_repo;
use helpers::{detect_package_managers, find_upward, has_github_workflows};
use next_steps::{build_next_command, build_next_steps};
use scope::{
    check_report_receipt_readiness, check_scan_limit, check_scan_scope, report_receipt_paths_ready,
};

const CONFIG_FILE_NAME: &str = "repopilot.toml";
const BASELINE_FILE_PATH: &str = ".repopilot/baseline.json";

struct DoctorRecommendationState {
    has_config: bool,
    config_readable: bool,
    has_repopilotignore: bool,
    has_baseline: bool,
    baseline_readable: bool,
    has_ci_config: bool,
    has_repopilot_ci_gate: bool,
    report_receipt_ready: bool,
    files_analyzed: usize,
    files_skipped_by_limit: usize,
}

fn build_recommendations(state: DoctorRecommendationState) -> Vec<String> {
    let mut recommendations = Vec::new();

    if !state.has_config {
        recommendations
            .push("Run `repopilot init` to create an explicit audit configuration.".to_string());
    } else if !state.config_readable {
        recommendations.push(
            "Fix `repopilot.toml` so RepoPilot can parse committed audit settings.".to_string(),
        );
    }

    if !state.has_repopilotignore {
        recommendations.push(
            "Add `.repopilotignore` for generated files, fixtures, snapshots, and vendor folders."
                .to_string(),
        );
    }

    if !state.has_baseline {
        recommendations.push(
            "Create a baseline with `repopilot baseline create .` before enforcing CI gates."
                .to_string(),
        );
    } else if !state.baseline_readable {
        recommendations.push(
            "Fix or regenerate `.repopilot/baseline.json` before using new-finding gates."
                .to_string(),
        );
    }

    if !state.has_repopilot_ci_gate {
        recommendations.push(
            "Add a RepoPilot CI gate with `--fail-on new-high` after committing a baseline."
                .to_string(),
        );
    } else if !state.has_ci_config {
        recommendations
            .push("Keep the RepoPilot gate in a committed CI workflow file.".to_string());
    }

    if !state.report_receipt_ready {
        recommendations.push(
            "Clear the default report or receipt path conflict before generating adoption evidence."
                .to_string(),
        );
    }

    if state.files_analyzed == 0 {
        recommendations.push(
            "Relax ignore rules or run with `--include-low-signal` if the target only contains tests/examples."
                .to_string(),
        );
    }

    if state.files_skipped_by_limit > 0 {
        recommendations.push(
            "Increase `--max-files` or remove the limit to audit the full repository scope."
                .to_string(),
        );
    }

    if recommendations.is_empty() {
        recommendations.push(
            "Repository adoption setup looks ready for regular scan/review usage.".to_string(),
        );
    }

    recommendations
}

pub fn build_doctor_report(
    path: &Path,
    explicit_config_path: Option<&Path>,
    config: &ScanConfig,
) -> io::Result<DoctorReport> {
    let summary = scan_path_with_config(path, config)?;
    let root = summary.root_path.clone();

    let config_path = explicit_config_path
        .filter(|path| path.is_file())
        .map(Path::to_path_buf)
        .or_else(|| find_upward(&root, CONFIG_FILE_NAME));

    let git_dir = find_upward(&root, ".git");
    let baseline_path = root.join(BASELINE_FILE_PATH);
    let github_workflows_dir = root.join(".github").join("workflows");

    let has_repopilotignore = summary.repopilotignore_path.is_some();
    let has_baseline = baseline_path.is_file();
    let has_github_workflows = has_github_workflows(&github_workflows_dir);
    let has_ci_config = has_ci_config(&root, &github_workflows_dir);
    let repopilot_ci = detect_repopilot_ci_config(&root, &github_workflows_dir);
    let config_readable = config_path.as_deref().is_some_and(config_file_is_readable);
    let baseline_readable = has_baseline && read_baseline(&baseline_path).is_ok();
    let report_receipt_ready = report_receipt_paths_ready(&root).is_ok();
    let package_managers = detect_package_managers(&root);

    let project = DoctorProject {
        languages: summary
            .languages
            .iter()
            .map(|language| language.name.clone())
            .collect(),
        frameworks: summary
            .detected_frameworks
            .iter()
            .map(|framework| framework.label())
            .collect(),
        package_managers,
        react_native_detected: summary.react_native.is_some(),
    };

    let scan = DoctorScanScope {
        files_discovered: summary.files_discovered,
        files_analyzed: summary.files_analyzed,
        files_skipped_low_signal: summary.files_skipped_low_signal,
        binary_files_skipped: summary.binary_files_skipped,
        large_files_skipped: summary.large_files_skipped,
        files_skipped_by_limit: summary.files_skipped_by_limit,
        files_skipped_repopilotignore: summary.files_skipped_repopilotignore,
    };

    let mut checks = vec![
        check_git_repo(git_dir.as_deref()),
        check_config(config_path.as_deref()),
    ];

    if let Some(path) = config_path.as_deref() {
        checks.push(check_config_readable(path));
    }

    checks.extend([
        check_repopilotignore(has_repopilotignore, summary.files_skipped_repopilotignore),
        check_baseline(has_baseline),
    ]);

    if has_baseline {
        checks.push(check_baseline_readable(&baseline_path));
    }

    checks.extend([
        check_ci_config(has_ci_config, has_github_workflows),
        check_repopilot_ci(&repopilot_ci, has_ci_config),
        check_report_receipt_readiness(&root),
        check_scan_scope(summary.files_analyzed),
        check_scan_limit(summary.files_skipped_by_limit),
    ]);

    let recommendations = build_recommendations(DoctorRecommendationState {
        has_config: config_path.is_some(),
        config_readable,
        has_repopilotignore,
        has_baseline,
        baseline_readable,
        has_ci_config,
        has_repopilot_ci_gate: repopilot_ci.has_gate,
        report_receipt_ready,
        files_analyzed: summary.files_analyzed,
        files_skipped_by_limit: summary.files_skipped_by_limit,
    });

    let next_steps = build_next_steps(
        path,
        config_path.is_some(),
        has_baseline,
        repopilot_ci.has_gate,
        summary.files_analyzed,
    );
    let next_command = next_steps
        .first()
        .map(|step| step.command.clone())
        .unwrap_or_else(|| build_next_command(path, has_baseline));

    Ok(DoctorReport {
        root_path: root.display().to_string(),
        project,
        scan,
        checks,
        recommendations,
        next_steps,
        next_command,
    })
}
