use crate::doctor::model::DoctorNextStep;
use std::path::Path;

const REPORT_FILE_PATH: &str = "repopilot-report.md";
const RECEIPT_FILE_PATH: &str = ".repopilot/receipt.json";

pub fn build_next_steps(
    path: &Path,
    has_config: bool,
    has_baseline: bool,
    has_repopilot_ci_gate: bool,
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
        command: format!("repopilot review {path}"),
        reason: "Review the current working-tree change before broader repository adoption."
            .to_string(),
    });

    steps.push(DoctorNextStep {
        command: format!(
            "repopilot scan {path} --format markdown --output {REPORT_FILE_PATH} --receipt {RECEIPT_FILE_PATH}"
        ),
        reason: "Generate a human-readable audit report plus a reproducible JSON receipt."
            .to_string(),
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

    if !has_repopilot_ci_gate {
        steps.push(DoctorNextStep {
            command: format!(
                "repopilot scan {path} --baseline .repopilot/baseline.json --fail-on new-high"
            ),
            reason: "Use this as the minimum CI gate after committing a baseline.".to_string(),
        });
    }

    steps.push(DoctorNextStep {
        command: format!("repopilot ai context {path} --budget 4k --output repopilot-context.md"),
        reason: "Create AI-ready remediation context without uploading source code.".to_string(),
    });

    steps
}

pub fn build_next_command(path: &Path, has_baseline: bool) -> String {
    let path = command_path(path);

    if has_baseline {
        format!("repopilot review {path} --baseline .repopilot/baseline.json --fail-on new-high")
    } else {
        format!("repopilot review {path}")
    }
}

pub fn command_path(path: &Path) -> String {
    let value = path.display().to_string();

    if value.is_empty() {
        ".".to_string()
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
                .any(|step| step.command.contains("--receipt .repopilot/receipt.json"))
        );
        assert!(
            steps
                .iter()
                .any(|step| step.command == "repopilot review .")
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
