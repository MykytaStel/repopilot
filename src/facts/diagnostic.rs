use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactDiagnostic {
    pub code: String,
    pub message: String,
    pub path: Option<PathBuf>,
}
