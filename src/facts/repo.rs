use super::{FactDiagnostic, FileFact};
use std::path::PathBuf;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RepoFacts {
    pub root: PathBuf,
    pub files: Vec<FileFact>,
    pub diagnostics: Vec<FactDiagnostic>,
}
