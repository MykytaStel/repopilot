use crate::findings::types::Finding;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq)]
pub enum MarkerKind {
    Todo,
    Fixme,
    Hack,
}

#[derive(Debug)]
pub struct Marker {
    pub kind: MarkerKind,
    pub line_number: usize,
    pub path: PathBuf,
    pub text: String,
}

#[derive(Debug, Default, Serialize, PartialEq, Eq)]
pub struct ScanSummary {
    pub root_path: PathBuf,
    pub files_count: usize,
    pub directories_count: usize,
    pub lines_of_code: usize,
    pub languages: Vec<LanguageSummary>,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct LanguageSummary {
    pub name: String,
    pub files_count: usize,
}
