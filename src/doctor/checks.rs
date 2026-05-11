use crate::doctor::model::{
    DoctorCheck, DoctorProject, DoctorReport, DoctorScanScope, DoctorStatus,
};
use crate::scan::config::ScanConfig;
use crate::scan::scanner::scan_path_with_config;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const CONFIG_FILE_NAME: &str = "repopilot.toml";
const BASELINE_FILE_PATH: &str = ".repopilot/baseline.json";

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

    let project = DoctorProject {
        languages: summary
            .languages
            .iter()
            .map(|language| language.name.clone())
            .collect(),
        frameworks: summary
            .detected_frameworks
            .iter()
            .map(|framework| format!("{framework:?}"))
            .collect(),
        react_native_detected: summary.react_native.is_some(),
    };

    let scan = DoctorScanScope {
        files_discovered: summary.files_discovered,
        files_analyzed: summary.files_count,
        files_skipped_low_signal: summary.files_skipped_low_signal,
        binary_files_skipped: summary.binary_files_skipped,
        large_files_skipped: summary.skipped_files_count,
        files_skipped_by_limit: summary.files_skipped_by_limit,
        files_skipped_repopilotignore: summary.files_skipped_repopilotignore,
    };

    let checks = vec![
        check_git_repo(git_dir.as_deref()),
        check_config(config_path.as_deref()),
        check_repopilotignore(has_repopilotignore, summary.files_skipped_repopilotignore),
        check_baseline(has_baseline),
        check_github_workflows(has_github_workflows),
        check_scan_scope(summary.files_count),
        check_scan_limit(summary.files_skipped_by_limit),
    ];

    let recommendations = build_recommendations(
        config_path.is_some(),
        has_repopilotignore,
        has_baseline,
        has_github_workflows,
        summary.files_count,
        summary.files_skipped_by_limit,
    );

    Ok(DoctorReport {
        root_path: root.display().to_string(),
        project,
        scan,
        checks,
        recommendations,
        next_command: build_next_command(path, has_baseline),
    })
}

fn check_git_repo(git_dir: Option<&Path>) -> DoctorCheck {
    match git_dir {
        Some(path) => DoctorCheck {
            id: "git".to_string(),
            status: DoctorStatus::Pass,
            title: "Git repository detected".to_string(),
            detail: format!("Found {}", path.display()),
        },
        None => DoctorCheck {
            id: "git".to_string(),
            status: DoctorStatus::Warn,
            title: "Git repository not detected".to_string(),
            detail: "Review and baseline workflows work best inside a Git repository.".to_string(),
        },
    }
}

fn check_config(config_path: Option<&Path>) -> DoctorCheck {
    match config_path {
        Some(path) => DoctorCheck {
            id: "config".to_string(),
            status: DoctorStatus::Pass,
            title: "RepoPilot config found".to_string(),
            detail: format!("Using {}", path.display()),
        },
        None => DoctorCheck {
            id: "config".to_string(),
            status: DoctorStatus::Warn,
            title: "RepoPilot config missing".to_string(),
            detail: "Run `repopilot init` to create repopilot.toml.".to_string(),
        },
    }
}

fn check_repopilotignore(has_repopilotignore: bool, skipped_files: usize) -> DoctorCheck {
    if has_repopilotignore {
        DoctorCheck {
            id: "repopilotignore".to_string(),
            status: DoctorStatus::Pass,
            title: ".repopilotignore found".to_string(),
            detail: format!("{skipped_files} files skipped by .repopilotignore."),
        }
    } else {
        DoctorCheck {
            id: "repopilotignore".to_string(),
            status: DoctorStatus::Warn,
            title: ".repopilotignore missing".to_string(),
            detail: "Add .repopilotignore to keep generated, fixture, and vendor files out of audit scope."
                .to_string(),
        }
    }
}

fn check_baseline(has_baseline: bool) -> DoctorCheck {
    if has_baseline {
        DoctorCheck {
            id: "baseline".to_string(),
            status: DoctorStatus::Pass,
            title: "Baseline found".to_string(),
            detail: format!("Found {BASELINE_FILE_PATH}."),
        }
    } else {
        DoctorCheck {
            id: "baseline".to_string(),
            status: DoctorStatus::Warn,
            title: "Baseline missing".to_string(),
            detail: "Run `repopilot baseline create .` before enabling CI failure gates."
                .to_string(),
        }
    }
}

fn check_github_workflows(has_github_workflows: bool) -> DoctorCheck {
    if has_github_workflows {
        DoctorCheck {
            id: "github_workflows".to_string(),
            status: DoctorStatus::Pass,
            title: "GitHub workflows found".to_string(),
            detail: "Repository has GitHub Actions workflow files.".to_string(),
        }
    } else {
        DoctorCheck {
            id: "github_workflows".to_string(),
            status: DoctorStatus::Warn,
            title: "GitHub workflows missing".to_string(),
            detail: "Add a RepoPilot workflow when you are ready to enforce scan/review checks."
                .to_string(),
        }
    }
}

fn check_scan_scope(files_analyzed: usize) -> DoctorCheck {
    if files_analyzed > 0 {
        DoctorCheck {
            id: "scan_scope".to_string(),
            status: DoctorStatus::Pass,
            title: "Scan scope is not empty".to_string(),
            detail: format!("{files_analyzed} files analyzed."),
        }
    } else {
        DoctorCheck {
            id: "scan_scope".to_string(),
            status: DoctorStatus::Fail,
            title: "Scan scope is empty".to_string(),
            detail: "No files were analyzed. Check ignore rules, path, file size limits, and low-signal filtering."
                .to_string(),
        }
    }
}

fn check_scan_limit(files_skipped_by_limit: usize) -> DoctorCheck {
    if files_skipped_by_limit == 0 {
        DoctorCheck {
            id: "scan_limit".to_string(),
            status: DoctorStatus::Pass,
            title: "No scan limit truncation".to_string(),
            detail: "No files were skipped by --max-files.".to_string(),
        }
    } else {
        DoctorCheck {
            id: "scan_limit".to_string(),
            status: DoctorStatus::Warn,
            title: "Scan was truncated by max-files".to_string(),
            detail: format!("{files_skipped_by_limit} files were skipped by the scan limit."),
        }
    }
}

fn build_recommendations(
    has_config: bool,
    has_repopilotignore: bool,
    has_baseline: bool,
    has_github_workflows: bool,
    files_analyzed: usize,
    files_skipped_by_limit: usize,
) -> Vec<String> {
    let mut recommendations = Vec::new();

    if !has_config {
        recommendations
            .push("Run `repopilot init` to create an explicit audit configuration.".to_string());
    }

    if !has_repopilotignore {
        recommendations.push(
            "Add `.repopilotignore` for generated files, fixtures, snapshots, and vendor folders."
                .to_string(),
        );
    }

    if !has_baseline {
        recommendations.push(
            "Create a baseline with `repopilot baseline create .` before enforcing CI gates."
                .to_string(),
        );
    }

    if !has_github_workflows {
        recommendations.push(
            "Add a GitHub Actions workflow for `repopilot scan` or `repopilot review`.".to_string(),
        );
    }

    if files_analyzed == 0 {
        recommendations.push(
            "Relax ignore rules or run with `--include-low-signal` if the target only contains tests/examples."
                .to_string(),
        );
    }

    if files_skipped_by_limit > 0 {
        recommendations.push(
            "Increase `--max-files` or remove the limit to audit the full repository scope."
                .to_string(),
        );
    }

    if recommendations.is_empty() {
        recommendations
            .push("Repository audit setup looks ready for regular scan/review usage.".to_string());
    }

    recommendations
}

fn build_next_command(path: &Path, has_baseline: bool) -> String {
    let path = command_path(path);

    if has_baseline {
        format!(
            "repopilot review {path} --base origin/main --baseline .repopilot/baseline.json --fail-on new-high"
        )
    } else {
        format!("repopilot scan {path} --format markdown --output repopilot-report.md")
    }
}

fn command_path(path: &Path) -> String {
    let value = path.display().to_string();

    if value.is_empty() {
        ".".to_string()
    } else {
        value
    }
}

fn find_upward(start: &Path, name: &str) -> Option<PathBuf> {
    let mut current = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };

    loop {
        let candidate = current.join(name);

        if candidate.exists() {
            return Some(candidate);
        }

        if !current.pop() {
            return None;
        }
    }
}

fn has_github_workflows(workflows_dir: &Path) -> bool {
    let Ok(entries) = fs::read_dir(workflows_dir) else {
        return false;
    };

    entries.filter_map(Result::ok).any(|entry| {
        let path = entry.path();

        path.is_file()
            && path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| matches!(extension, "yml" | "yaml"))
    })
}
