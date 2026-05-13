use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DoctorReport {
    pub root_path: String,
    pub project: DoctorProject,
    pub scan: DoctorScanScope,
    pub checks: Vec<DoctorCheck>,
    pub recommendations: Vec<String>,
    pub next_steps: Vec<DoctorNextStep>,
    pub next_command: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DoctorProject {
    pub languages: Vec<String>,
    pub frameworks: Vec<String>,
    pub package_managers: Vec<String>,
    pub react_native_detected: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DoctorScanScope {
    pub files_discovered: usize,
    pub files_analyzed: usize,
    pub files_skipped_low_signal: usize,
    pub binary_files_skipped: usize,
    pub large_files_skipped: usize,
    pub files_skipped_by_limit: usize,
    pub files_skipped_repopilotignore: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DoctorCheck {
    pub id: String,
    pub status: DoctorStatus,
    pub title: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DoctorNextStep {
    pub command: String,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DoctorStatus {
    Pass,
    Warn,
    Fail,
}

impl DoctorStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Warn => "WARN",
            Self::Fail => "FAIL",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::Pass => "✅",
            Self::Warn => "⚠️",
            Self::Fail => "❌",
        }
    }
}
