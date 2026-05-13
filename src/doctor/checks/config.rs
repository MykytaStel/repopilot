use crate::config::loader::parse_config;
use crate::doctor::model::{DoctorCheck, DoctorStatus};
use std::fs;
use std::path::Path;

pub fn check_config(config_path: Option<&Path>) -> DoctorCheck {
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

pub fn check_config_readable(config_path: &Path) -> DoctorCheck {
    match read_config_file(config_path) {
        Ok(()) => DoctorCheck {
            id: "config_readable".to_string(),
            status: DoctorStatus::Pass,
            title: "RepoPilot config is readable".to_string(),
            detail: format!("Parsed {} successfully.", config_path.display()),
        },
        Err(reason) => DoctorCheck {
            id: "config_readable".to_string(),
            status: DoctorStatus::Fail,
            title: "RepoPilot config is not readable".to_string(),
            detail: reason,
        },
    }
}

pub fn check_repopilotignore(has_repopilotignore: bool, skipped_files: usize) -> DoctorCheck {
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

pub fn read_config_file(path: &Path) -> Result<(), String> {
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("Failed to read {}: {error}", path.display()))?;

    parse_config(&contents, Some(path))
        .map(|_| ())
        .map_err(|error| error.to_string())
}

pub fn config_file_is_readable(path: &Path) -> bool {
    read_config_file(path).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn flags_invalid_config_as_unreadable() {
        let dir = tempdir().expect("tempdir should be created");
        let config_path = dir.path().join("repopilot.toml");
        fs::write(&config_path, "[scan").expect("config should be written");

        let check = check_config_readable(&config_path);

        assert_eq!(check.status, DoctorStatus::Fail);
        assert_eq!(check.id, "config_readable");
    }
}
