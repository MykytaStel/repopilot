use crate::frameworks::DetectedFramework;
use crate::frameworks::FrameworkProject;
use crate::frameworks::ReactNativeArchitectureProfile;
use crate::scan::types::LanguageSummary;
use std::path::PathBuf;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ScanFacts {
    pub root_path: PathBuf,
    pub files_count: usize,
    pub directories_count: usize,
    pub lines_of_code: usize,
    pub skipped_files_count: usize,
    pub skipped_bytes: u64,
    pub languages: Vec<LanguageSummary>,
    pub files: Vec<FileFacts>,
    pub detected_frameworks: Vec<DetectedFramework>,
    pub framework_projects: Vec<FrameworkProject>,
    pub react_native: Option<ReactNativeArchitectureProfile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFacts {
    pub path: PathBuf,
    pub language: Option<String>,
    pub lines_of_code: usize,
    pub branch_count: usize,
    pub imports: Vec<String>,
    pub content: String,
    /// True if the file contains a `#[cfg(test)]` block (Rust inline unit tests).
    /// Computed while content is available; preserved after content is dropped.
    pub has_inline_tests: bool,
}
