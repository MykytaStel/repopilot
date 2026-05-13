use crate::doctor::model::{DoctorCheck, DoctorStatus};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RepopilotCiDetection {
    pub has_repopilot: bool,
    pub has_gate: bool,
    pub path: Option<PathBuf>,
}

pub fn check_ci_config(has_ci_config: bool, has_github_workflows: bool) -> DoctorCheck {
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

pub fn check_repopilot_ci(detection: &RepopilotCiDetection, has_ci_config: bool) -> DoctorCheck {
    if detection.has_gate {
        return DoctorCheck {
            id: "repopilot_ci".to_string(),
            status: DoctorStatus::Pass,
            title: "RepoPilot CI gate configured".to_string(),
            detail: format!(
                "Found a RepoPilot failure gate in {}.",
                detection
                    .path
                    .as_deref()
                    .map(Path::display)
                    .map(|display| display.to_string())
                    .unwrap_or_else(|| "CI configuration".to_string())
            ),
        };
    }

    if detection.has_repopilot {
        return DoctorCheck {
            id: "repopilot_ci".to_string(),
            status: DoctorStatus::Warn,
            title: "RepoPilot CI integration has no gate".to_string(),
            detail: format!(
                "Found RepoPilot in {}, but no `--fail-on`/`fail-on` threshold was detected.",
                detection
                    .path
                    .as_deref()
                    .map(Path::display)
                    .map(|display| display.to_string())
                    .unwrap_or_else(|| "CI configuration".to_string())
            ),
        };
    }

    DoctorCheck {
        id: "repopilot_ci".to_string(),
        status: DoctorStatus::Warn,
        title: "RepoPilot CI gate missing".to_string(),
        detail: if has_ci_config {
            "Generic CI exists, but no RepoPilot scan/review failure gate was detected.".to_string()
        } else {
            "Add a RepoPilot scan or review gate when you are ready to enforce new findings."
                .to_string()
        },
    }
}

pub fn has_ci_config(root: &Path, github_workflows_dir: &Path) -> bool {
    use super::helpers::has_github_workflows;

    has_github_workflows(github_workflows_dir)
        || root.join(".gitlab-ci.yml").is_file()
        || root.join("azure-pipelines.yml").is_file()
        || root.join(".circleci").join("config.yml").is_file()
        || root.join("Jenkinsfile").is_file()
}

pub fn detect_repopilot_ci_config(root: &Path, github_workflows_dir: &Path) -> RepopilotCiDetection {
    let mut integration_only = RepopilotCiDetection::default();

    for path in ci_candidate_paths(root, github_workflows_dir) {
        let Ok(contents) = fs::read_to_string(&path) else {
            continue;
        };
        let lower = contents.to_ascii_lowercase();

        if !lower.contains("repopilot") {
            continue;
        }

        let detection = RepopilotCiDetection {
            has_repopilot: true,
            has_gate: lower.contains("--fail-on")
                || lower.contains("fail-on:")
                || lower.contains("fail_on:"),
            path: Some(path),
        };

        if detection.has_gate {
            return detection;
        }

        if !integration_only.has_repopilot {
            integration_only = detection;
        }
    }

    integration_only
}

pub fn ci_candidate_paths(root: &Path, github_workflows_dir: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(entries) = fs::read_dir(github_workflows_dir) {
        paths.extend(
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| {
                    path.is_file()
                        && path
                            .extension()
                            .and_then(|extension| extension.to_str())
                            .is_some_and(|extension| matches!(extension, "yml" | "yaml"))
                }),
        );
    }

    for path in [
        root.join(".gitlab-ci.yml"),
        root.join("azure-pipelines.yml"),
        root.join(".circleci").join("config.yml"),
        root.join("Jenkinsfile"),
    ] {
        if path.is_file() {
            paths.push(path);
        }
    }

    paths
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn detects_github_actions_as_ci_config() {
        let dir = tempdir().expect("tempdir should be created");
        let workflows = dir.path().join(".github").join("workflows");
        fs::create_dir_all(&workflows).expect("workflow dir should be created");
        fs::write(workflows.join("ci.yml"), "name: CI").expect("workflow should be written");

        assert!(has_ci_config(dir.path(), &workflows));
    }

    #[test]
    fn detects_generic_ci_without_repopilot_gate() {
        let dir = tempdir().expect("tempdir should be created");
        let workflows = dir.path().join(".github").join("workflows");
        fs::create_dir_all(&workflows).expect("workflow dir should be created");
        fs::write(
            workflows.join("ci.yml"),
            "name: CI\njobs:\n  test:\n    runs-on: ubuntu-latest\n",
        )
        .expect("workflow should be written");

        let detection = detect_repopilot_ci_config(dir.path(), &workflows);
        let check = check_repopilot_ci(&detection, true);

        assert!(!detection.has_repopilot);
        assert_eq!(check.status, DoctorStatus::Warn);
        assert_eq!(check.id, "repopilot_ci");
    }

    #[test]
    fn detects_repopilot_ci_gate() {
        let dir = tempdir().expect("tempdir should be created");
        let workflows = dir.path().join(".github").join("workflows");
        fs::create_dir_all(&workflows).expect("workflow dir should be created");
        fs::write(
            workflows.join("repopilot.yml"),
            "name: RepoPilot\njobs:\n  scan:\n    steps:\n      - run: repopilot scan . --baseline .repopilot/baseline.json --fail-on new-high\n",
        )
        .expect("workflow should be written");

        let detection = detect_repopilot_ci_config(dir.path(), &workflows);
        let check = check_repopilot_ci(&detection, true);

        assert!(detection.has_repopilot);
        assert!(detection.has_gate);
        assert_eq!(check.status, DoctorStatus::Pass);
    }
}
