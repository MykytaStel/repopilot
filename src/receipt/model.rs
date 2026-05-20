use crate::findings::feedback::LocalSuppression;
use crate::report::schema::ReportEnvelope;
use crate::scan::types::DiagnosticSeverity;
use serde::Serialize;

pub const AUDIT_RECEIPT_SCHEMA_VERSION: u32 = 5;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AuditReceipt {
    pub schema_version: u32,
    pub report: ReportEnvelope,
    pub tool: String,
    pub version: String,
    pub generated_at: String,
    pub root_path: String,
    pub git: ReceiptGit,
    pub scope: ReceiptScope,
    pub findings: ReceiptFindings,
    pub local_feedback: Option<ReceiptLocalFeedback>,
    pub languages: Vec<ReceiptLanguage>,
    pub diagnostics: Vec<ReceiptDiagnostic>,
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
    pub mode: String,
    pub base_ref: Option<String>,
    pub changed_files_count: usize,
    pub repo_level_rules_included: bool,
    pub files_discovered: usize,
    pub files_analyzed: usize,
    pub directories_count: usize,
    pub non_empty_lines: usize,
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
    pub hidden_suggestions_count: usize,
    pub hidden_suggestions: Vec<ReceiptHiddenSuggestion>,
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub info: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ReceiptHiddenSuggestion {
    pub intent: String,
    pub rule_id: String,
    pub category: String,
    pub reason: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ReceiptLocalFeedback {
    pub feedback_path: Option<String>,
    pub suppressions_loaded: usize,
    pub suppressed_findings_count: usize,
    pub unmatched_suppressions_count: usize,
    pub invalid_suppressions_count: usize,
    pub unmatched_suppressions: Vec<LocalSuppression>,
    pub parse_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ReceiptLanguage {
    pub name: String,
    pub files_analyzed: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ReceiptDiagnostic {
    pub code: String,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub path: Option<String>,
}
