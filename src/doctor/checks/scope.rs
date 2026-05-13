use crate::doctor::model::{DoctorCheck, DoctorStatus};
use std::path::Path;

const REPORT_FILE_PATH: &str = "repopilot-report.md";
const RECEIPT_FILE_PATH: &str = ".repopilot/receipt.json";

pub fn check_scan_scope(files_analyzed: usize) -> DoctorCheck {
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

pub fn check_scan_limit(files_skipped_by_limit: usize) -> DoctorCheck {
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

pub fn check_report_receipt_readiness(root: &Path) -> DoctorCheck {
    match report_receipt_paths_ready(root) {
        Ok(detail) => DoctorCheck {
            id: "report_receipt".to_string(),
            status: DoctorStatus::Pass,
            title: "Report and receipt paths are ready".to_string(),
            detail,
        },
        Err(detail) => DoctorCheck {
            id: "report_receipt".to_string(),
            status: DoctorStatus::Fail,
            title: "Report or receipt path is blocked".to_string(),
            detail,
        },
    }
}

pub fn report_receipt_paths_ready(root: &Path) -> Result<String, String> {
    let report_path = root.join(REPORT_FILE_PATH);
    let repopilot_dir = root.join(".repopilot");
    let receipt_path = root.join(RECEIPT_FILE_PATH);

    if report_path.is_dir() {
        return Err(format!(
            "{} is a directory; choose another report output path.",
            report_path.display()
        ));
    }

    if repopilot_dir.is_file() {
        return Err(format!(
            "{} is a file; RepoPilot needs a directory for receipt output.",
            repopilot_dir.display()
        ));
    }

    if receipt_path.is_dir() {
        return Err(format!(
            "{} is a directory; choose another receipt output path.",
            receipt_path.display()
        ));
    }

    Ok(format!(
        "Default adoption outputs are available: `{REPORT_FILE_PATH}` and `{RECEIPT_FILE_PATH}`."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn report_and_receipt_paths_are_ready_by_default() {
        let dir = tempdir().expect("tempdir should be created");

        let check = check_report_receipt_readiness(dir.path());

        assert_eq!(check.status, DoctorStatus::Pass);
        assert!(check.detail.contains(RECEIPT_FILE_PATH));
    }
}
