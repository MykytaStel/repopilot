use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AuditReceipt {
    pub tool: String,
    pub version: String,
    pub generated_at: String,
    pub root_path: String,
    pub git: ReceiptGit,
    pub scope: ReceiptScope,
    pub findings: ReceiptFindings,
    pub languages: Vec<ReceiptLanguage>,
    pub health_score: u8,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ReceiptGit {
    pub is_git_repo: bool,
    pub branch: Option<String>,
    pub commit: Option<String>,
    pub dirty: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ReceiptScope {
    pub files_discovered: usize,
    pub files_analyzed: usize,
    pub directories_count: usize,
    pub lines_of_code: usize,
    pub files_skipped_low_signal: usize,
    pub binary_files_skipped: usize,
    pub large_files_skipped: usize,
    pub files_skipped_by_limit: usize,
    pub files_skipped_repopilotignore: usize,
    pub skipped_bytes: u64,
    pub repopilotignore_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ReceiptFindings {
    pub total: usize,
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub info: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ReceiptLanguage {
    pub name: String,
    pub files_count: usize,
}
