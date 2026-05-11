use crate::frameworks::DetectedFramework;
use crate::frameworks::FrameworkProject;
use crate::frameworks::ReactNativeArchitectureProfile;
use crate::scan::types::LanguageSummary;
use std::path::PathBuf;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ScanFacts {
    pub root_path: PathBuf,
    pub files_discovered: usize,
    pub files_count: usize,
    pub directories_count: usize,
    pub lines_of_code: usize,
    pub skipped_files_count: usize,
    pub files_skipped_low_signal: usize,
    pub binary_files_skipped: usize,
    pub skipped_bytes: u64,
    pub languages: Vec<LanguageSummary>,
    pub files: Vec<FileFacts>,
    pub detected_frameworks: Vec<DetectedFramework>,
    pub framework_projects: Vec<FrameworkProject>,
    pub react_native: Option<ReactNativeArchitectureProfile>,
    pub files_skipped_by_limit: usize,
    pub files_skipped_repopilotignore: usize,
    pub repopilotignore_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFacts {
    pub path: PathBuf,
    pub language: Option<String>,
    pub lines_of_code: usize,
    pub branch_count: usize,
    pub imports: Vec<String>,
    /// Source text while file audits are running. `None` means the file was skipped,
    /// binary/unreadable, or retained only as summary metadata after auditing.
    pub content: Option<String>,
    /// True if the file contains a `#[cfg(test)]` block (Rust inline unit tests).
    /// Computed while content is available; preserved after content is dropped.
    pub has_inline_tests: bool,
}
