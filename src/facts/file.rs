use super::{FactConfidence, FactSource};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFact {
    pub path: PathBuf,
    pub language: Option<String>,
    pub non_empty_lines: usize,
    pub source: FactSource,
    pub confidence: FactConfidence,
}
