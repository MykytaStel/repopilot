use crate::findings::types::Finding;
use crate::graph::CouplingGraph;
use crate::graph::context::{ContextGraphCacheInfo, RepoContextGraph};
use crate::review::diff::ChangedFile;
use crate::scan::cache::ScanCache;
use crate::scan::facts::{FileFacts, ScanFacts};
use crate::scan::types::{ScanCacheTelemetry, ScanDiagnostic};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub(super) struct ChangedDiscoveryStage {
    pub repo_root: PathBuf,
    pub changed_files: Vec<ChangedFile>,
    pub elapsed_us: u64,
}

pub(super) struct ChangedFileAnalysisStage {
    pub facts: ScanFacts,
    pub findings: Vec<Finding>,
    pub graph_patch_files: Vec<FileFacts>,
    pub cache: ScanCache,
    pub cache_telemetry: ScanCacheTelemetry,
    pub changed_file_reasons: BTreeMap<String, usize>,
    pub elapsed_us: u64,
    pub parse_us: u64,
}

pub(super) struct ChangedRepoContextStage {
    pub repo_context: ScanFacts,
    pub coupling_graph: CouplingGraph,
    pub context_graph: RepoContextGraph,
    pub cache_info: ContextGraphCacheInfo,
    pub diagnostics: Vec<ScanDiagnostic>,
    pub elapsed_us: u64,
}

pub(super) struct ChangedFindingPipelineStage {
    pub post_scan_audits_us: u64,
    pub enrichment_us: u64,
    pub risk_scoring_us: u64,
    pub contract_validation_us: u64,
    pub diagnostics: Vec<ScanDiagnostic>,
    pub signal_quality: crate::findings::quality::SignalQualitySummary,
}
