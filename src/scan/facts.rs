use crate::scan::types::LanguageSummary;
use std::path::PathBuf;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ScanFacts {
    pub root_path: PathBuf,
    pub files_count: usize,
    pub directories_count: usize,
    pub lines_of_code: usize,
    pub languages: Vec<LanguageSummary>,
    pub files: Vec<FileFacts>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFacts {
    pub path: PathBuf,
    pub language: Option<String>,
    pub lines_of_code: usize,
    pub content: String,
}
