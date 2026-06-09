use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphDiagnostic {
    pub code: String,
    pub message: String,
    pub path: Option<PathBuf>,
}
