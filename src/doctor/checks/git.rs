use crate::doctor::model::{DoctorCheck, DoctorStatus};
use std::path::Path;

pub fn check_git_repo(git_dir: Option<&Path>) -> DoctorCheck {
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
