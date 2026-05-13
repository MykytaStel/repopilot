use crate::doctor::model::{
    DoctorCheck, DoctorNextStep, DoctorProject, DoctorReport, DoctorScanScope, DoctorStatus,
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
    let has_ci_config = has_ci_config(&root, &github_workflows_dir);
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
        check_ci_config(has_ci_config, has_github_workflows),
        check_scan_scope(summary.files_count),
        check_scan_limit(summary.files_skipped_by_limit),
    ];

    let recommendations = build_recommendations(
        config_path.is_some(),
        has_repopilotignore,
        has_baseline,
        has_ci_config,
        summary.files_count,
        summary.files_skipped_by_limit,
    );

    let next_steps = build_next_steps(
        path,
        config_path.is_some(),
        has_baseline,
        has_ci_config,
        summary.files_count,
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

fn check_ci_config(has_ci_config: bool, has_github_workflows: bool) -> DoctorCheck {
    match (has_ci_config, has_github_workflows) {
        (true, true) => DoctorCheck {
            id: "ci".to_string(),
            status: DoctorStatus::Pass,
            title: "CI workflow detected".to_string(),
            detail: "Repository has GitHub Actions workflow files.".to_string(),
        },
        (true, false) => DoctorCheck {
            id: "ci".to_string(),
            status: DoctorStatus::Pass,
            title: "CI config detected".to_string(),
            detail: "Repository has a CI configuration file outside GitHub Actions.".to_string(),
        },
        (false, _) => DoctorCheck {
            id: "ci".to_string(),
            status: DoctorStatus::Warn,
            title: "CI config missing".to_string(),
            detail: "Add a RepoPilot workflow when you are ready to enforce scan/review checks."
                .to_string(),
        },
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
    has_ci_config: bool,
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

    if !has_ci_config {
        recommendations
            .push("Add a CI workflow for `repopilot scan` or `repopilot review`.".to_string());
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

fn build_next_steps(
    path: &Path,
    has_config: bool,
    has_baseline: bool,
    has_ci_config: bool,
    files_analyzed: usize,
) -> Vec<DoctorNextStep> {
    let path = command_path(path);
    let mut steps = Vec::new();

    if !has_config {
        steps.push(DoctorNextStep {
            command: "repopilot init".to_string(),
            reason: "Create an explicit repopilot.toml before tuning thresholds or CI gates."
                .to_string(),
        });
    }

    if files_analyzed == 0 {
        steps.push(DoctorNextStep {
            command: format!("repopilot doctor {path} --include-low-signal"),
            reason: "Re-run readiness diagnostics with low-signal paths included because the current scan scope is empty."
                .to_string(),
        });
    }

    steps.push(DoctorNextStep {
        command: format!("repopilot scan {path} --format markdown --output repopilot-report.md"),
        reason: "Generate a human-readable first audit report.".to_string(),
    });

    steps.push(DoctorNextStep {
        command: format!("repopilot ai context {path} --budget 4k --output repopilot-context.md"),
        reason: "Create AI-ready remediation context without uploading source code.".to_string(),
    });

    if has_baseline {
        steps.push(DoctorNextStep {
            command: format!(
                "repopilot review {path} --base origin/main --baseline .repopilot/baseline.json --fail-on new-high"
            ),
            reason: "Review only changed-code findings against the accepted baseline.".to_string(),
        });
    } else {
        steps.push(DoctorNextStep {
            command: format!("repopilot baseline create {path}"),
            reason:
                "Accept existing findings as technical debt before enabling new-finding CI gates."
                    .to_string(),
        });
    }

    if !has_ci_config {
        steps.push(DoctorNextStep {
            command: format!(
                "repopilot scan {path} --baseline .repopilot/baseline.json --fail-on new-high"
            ),
            reason: "Use this command in CI after committing a baseline.".to_string(),
        });
    }

    steps
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

fn detect_package_managers(root: &Path) -> Vec<String> {
    let mut managers = Vec::new();

    if root.join("Cargo.toml").is_file() {
        managers.push("Cargo".to_string());
    }

    if root.join("package.json").is_file() {
        if root.join("pnpm-lock.yaml").is_file() {
            managers.push("pnpm".to_string());
        } else if root.join("yarn.lock").is_file() {
            managers.push("Yarn".to_string());
        } else if root.join("bun.lockb").is_file() || root.join("bun.lock").is_file() {
            managers.push("Bun".to_string());
        } else if root.join("package-lock.json").is_file() {
            managers.push("npm".to_string());
        } else {
            managers.push("Node package.json".to_string());
        }
    }

    if root.join("pyproject.toml").is_file() {
        managers.push("Python pyproject".to_string());
    } else if root.join("requirements.txt").is_file() {
        managers.push("pip requirements".to_string());
    }

    if root.join("go.mod").is_file() {
        managers.push("Go modules".to_string());
    }

    if root.join("Gemfile").is_file() {
        managers.push("Bundler".to_string());
    }

    if root.join("composer.json").is_file() {
        managers.push("Composer".to_string());
    }

    if root.join("pom.xml").is_file()
        || root.join("build.gradle").is_file()
        || root.join("build.gradle.kts").is_file()
    {
        managers.push("JVM build".to_string());
    }

    if root.join("Package.swift").is_file() {
        managers.push("Swift Package Manager".to_string());
    }

    managers
}

fn has_ci_config(root: &Path, github_workflows_dir: &Path) -> bool {
    has_github_workflows(github_workflows_dir)
        || root.join(".gitlab-ci.yml").is_file()
        || root.join("azure-pipelines.yml").is_file()
        || root.join(".circleci").join("config.yml").is_file()
        || root.join("Jenkinsfile").is_file()
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn detects_pnpm_package_manager() {
        let dir = tempdir().expect("tempdir should be created");
        fs::write(dir.path().join("package.json"), "{}").expect("package.json should be written");
        fs::write(dir.path().join("pnpm-lock.yaml"), "lockfileVersion: 9")
            .expect("pnpm lockfile should be written");

        let managers = detect_package_managers(dir.path());

        assert_eq!(managers, vec!["pnpm"]);
    }

    #[test]
    fn detects_github_actions_as_ci_config() {
        let dir = tempdir().expect("tempdir should be created");
        let workflows = dir.path().join(".github").join("workflows");
        fs::create_dir_all(&workflows).expect("workflow dir should be created");
        fs::write(workflows.join("ci.yml"), "name: CI").expect("workflow should be written");

        assert!(has_ci_config(dir.path(), &workflows));
    }

    #[test]
    fn next_steps_start_with_init_when_config_is_missing() {
        let steps = build_next_steps(Path::new("."), false, false, false, 1);

        assert_eq!(
            steps.first().map(|step| step.command.as_str()),
            Some("repopilot init")
        );
        assert!(
            steps
                .iter()
                .any(|step| step.command == "repopilot baseline create .")
        );
    }

    #[test]
    fn next_steps_use_review_when_baseline_exists() {
        let steps = build_next_steps(Path::new("."), true, true, true, 1);

        assert!(steps.iter().any(|step| {
            step.command
                == "repopilot review . --base origin/main --baseline .repopilot/baseline.json --fail-on new-high"
        }));
    }
}
