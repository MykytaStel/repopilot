use crate::baseline::reader::read_baseline;
use crate::doctor::model::{DoctorCheck, DoctorStatus};
use std::path::Path;

const BASELINE_FILE_PATH: &str = ".repopilot/baseline.json";

pub fn check_baseline(has_baseline: bool) -> DoctorCheck {
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

pub fn check_baseline_readable(baseline_path: &Path) -> DoctorCheck {
    match read_baseline(baseline_path) {
        Ok(baseline) => DoctorCheck {
            id: "baseline_readable".to_string(),
            status: DoctorStatus::Pass,
            title: "Baseline is readable".to_string(),
            detail: format!(
                "Parsed {} with {} accepted findings.",
                baseline_path.display(),
                baseline.findings.len()
            ),
        },
        Err(error) => DoctorCheck {
            id: "baseline_readable".to_string(),
            status: DoctorStatus::Fail,
            title: "Baseline is not readable".to_string(),
            detail: error.to_string().replace('\n', " "),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn flags_invalid_baseline_as_unreadable() {
        let dir = tempdir().expect("tempdir should be created");
        let baseline_dir = dir.path().join(".repopilot");
        fs::create_dir(&baseline_dir).expect("baseline dir should be created");
        let baseline_path = baseline_dir.join("baseline.json");
        fs::write(&baseline_path, "{").expect("baseline should be written");

        let check = check_baseline_readable(&baseline_path);

        assert_eq!(check.status, DoctorStatus::Fail);
        assert_eq!(check.id, "baseline_readable");
    }
}
