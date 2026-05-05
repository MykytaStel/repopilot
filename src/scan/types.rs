use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct ScanSummary {
    pub root_path: PathBuf,
    pub files_count: usize,
    pub directories_count: usize,
    pub lines_of_code: usize,
    pub languages: Vec<LanguageSummary>,
    pub markers: Vec<CodeMarker>,
}

#[derive(Debug)]
pub struct LanguageSummary {
    pub name: String,
    pub files_count: usize,
}

#[derive(Debug)]
pub struct CodeMarker {
    pub kind: MarkerKind,
    pub path: PathBuf,
    pub line_number: usize,
    pub text: String,
}

#[derive(Debug)]
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
