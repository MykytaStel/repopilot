use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, PartialEq, Eq)]
pub struct ScanSummary {
    pub root_path: PathBuf,
    pub files_count: usize,
    pub directories_count: usize,
    pub lines_of_code: usize,
    pub languages: Vec<LanguageSummary>,
    pub markers: Vec<CodeMarker>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct LanguageSummary {
    pub name: String,
    pub files_count: usize,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct CodeMarker {
    pub kind: MarkerKind,
    pub path: PathBuf,
    pub line_number: usize,
    pub text: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MarkerKind {
    Todo,
    Fixme,
    Hack,
}

impl std::fmt::Display for MarkerKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MarkerKind::Todo => write!(formatter, "TODO"),
            MarkerKind::Fixme => write!(formatter, "FIXME"),
            MarkerKind::Hack => write!(formatter, "HACK"),
        }
    }
}
