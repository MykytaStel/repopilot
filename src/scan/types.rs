use crate::findings::types::Finding;
use crate::frameworks::DetectedFramework;
use crate::frameworks::ReactNativeArchitectureProfile;
use crate::graph::CouplingGraph;
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanSummary {
    pub root_path: PathBuf,
    pub files_count: usize,
    pub directories_count: usize,
    pub lines_of_code: usize,
    #[serde(default)]
    pub skipped_files_count: usize,
    #[serde(default)]
    pub skipped_bytes: u64,
    pub languages: Vec<LanguageSummary>,
    pub findings: Vec<Finding>,
    #[serde(default)]
    pub detected_frameworks: Vec<DetectedFramework>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub react_native: Option<ReactNativeArchitectureProfile>,
    #[serde(default)]
    pub coupling_graph: Option<CouplingGraph>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LanguageSummary {
    pub name: String,
    pub files_count: usize,
}
