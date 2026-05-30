use super::changed_cache::{
    CacheDecision, cache_decision, normalize_per_file_paths, record_cached_file,
};
use super::changed_git::collect_changed_scope;
use super::changed_telemetry::{
    change_status_label, finalize_cache_telemetry, record_skipped_cache_file,
};
use super::collection;
use super::file::{SkipReason, process_file_with_content};
use super::summary::{self, ScanSummaryParts, build_language_summary, build_scan_summary};
use crate::audits::pipeline::build_file_audits;
use crate::findings::enrichment::enrich_findings_timed;
use crate::findings::quality::{
    SignalQualitySummary, summarize_signal_quality_with_contract_violations,
};
use crate::findings::types::Finding;
use crate::frameworks::{
    DetectedFramework, detect_framework_projects, detect_frameworks,
    detect_react_native_architecture,
};
use crate::graph::context::{
    ContextGraphCacheInfo, RepoContextGraph, context_graph_cache_miss, load_repo_context_graph,
    summarize_context_graph, write_repo_context_graph,
};
use crate::graph::{CouplingGraph, build_coupling_graph};
use crate::review::diff::{ChangeStatus, ChangedFile};
use crate::risk::{apply_cluster_overlay, apply_graph_overlay, assess_findings};
use crate::scan::cache::{
    FileRoleEntry, FindingsEntry, ScanCache, config_fingerprint, file_hash_entry,
    relative_cache_path,
};
use crate::scan::config::ScanConfig;
use crate::scan::facts::{FileFacts, ScanFacts};
use crate::scan::types::cache_diagnostic;
use crate::scan::types::{
    ChangedFileCacheTelemetry, ScanCacheTelemetry, ScanDiagnostic, ScanMode, ScanSummary,
    ScanTimings,
};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

pub fn scan_changed_with_config(
    path: &Path,
    config: &ScanConfig,
    base_ref: Option<&str>,
) -> io::Result<ScanSummary> {
    ChangedScanEngine::new(path, config, base_ref).run()
}

struct ChangedScanEngine<'a> {
    path: &'a Path,
    config: &'a ScanConfig,
    base_ref: Option<&'a str>,
}

struct ChangedDiscoveryStage {
    repo_root: PathBuf,
    changed_files: Vec<ChangedFile>,
    elapsed_us: u64,
}

struct ChangedFileAnalysisStage {
    facts: ScanFacts,
    findings: Vec<Finding>,
    graph_patch_files: Vec<FileFacts>,
    cache: ScanCache,
    cache_telemetry: ScanCacheTelemetry,
    changed_file_reasons: BTreeMap<String, usize>,
    elapsed_us: u64,
}

struct ChangedRepoContextStage {
    repo_context: ScanFacts,
    coupling_graph: CouplingGraph,
    context_graph: RepoContextGraph,
    cache_info: ContextGraphCacheInfo,
    diagnostics: Vec<ScanDiagnostic>,
    elapsed_us: u64,
}

struct ChangedFindingPipelineStage {
    enrichment_us: u64,
    risk_scoring_us: u64,
    contract_validation_us: u64,
    diagnostics: Vec<crate::scan::types::ScanDiagnostic>,
    signal_quality: SignalQualitySummary,
}

impl<'a> ChangedScanEngine<'a> {
    fn new(path: &'a Path, config: &'a ScanConfig, base_ref: Option<&'a str>) -> Self {
        Self {
            path,
            config,
            base_ref,
        }
    }

    fn run(self) -> io::Result<ScanSummary> {
        let start = Instant::now();
        let discovery = self.run_discovery()?;
        if discovery.changed_files.is_empty() {
            return self.finalize_empty_changed(start, discovery);
        }
        let mut file_stage = self.run_file_analysis(&discovery)?;
        let repo_stage = self.run_repo_context(
            &discovery,
            &mut file_stage.facts,
            &file_stage.graph_patch_files,
        )?;
        let enrichment_us = enrich_findings_timed(&mut file_stage.findings, &discovery.repo_root);
        let risk_scoring_us = self.score_findings(&repo_stage, &mut file_stage.findings);
        let contract_stage =
            super::contract_stage::validate_finding_contract_stage(&file_stage.findings);
        let signal_quality = summarize_signal_quality_with_contract_violations(
            &file_stage.findings,
            contract_stage.report.violations.len(),
        );
        let finding_pipeline = ChangedFindingPipelineStage {
            enrichment_us,
            risk_scoring_us,
            contract_validation_us: contract_stage.elapsed_us,
            diagnostics: contract_stage.diagnostics,
            signal_quality,
        };

        self.finalize_report(start, discovery, file_stage, repo_stage, finding_pipeline)
    }

    fn run_discovery(&self) -> io::Result<ChangedDiscoveryStage> {
        let start = Instant::now();
        let changed_scope = collect_changed_scope(self.path, self.base_ref)?;
        Ok(ChangedDiscoveryStage {
            repo_root: changed_scope.repo_root,
            changed_files: changed_scope.changed_files,
            elapsed_us: start.elapsed().as_micros() as u64,
        })
    }
}

mod file_analysis;
mod finalize;
mod repo_context;

fn detect_react_native_profile(
    facts: &ScanFacts,
) -> Option<crate::frameworks::ReactNativeArchitectureProfile> {
    if facts
        .detected_frameworks
        .iter()
        .any(|f| matches!(f, DetectedFramework::ReactNative { .. }))
    {
        let profile = detect_react_native_architecture(&facts.root_path);
        if profile.detected {
            return Some(profile);
        }
    }
    None
}

fn relative_coupling_graph(graph: CouplingGraph, repo_root: &Path) -> CouplingGraph {
    CouplingGraph {
        edges: graph
            .edges
            .into_iter()
            .map(|(source, targets)| {
                (
                    PathBuf::from(relative_cache_path(repo_root, &source)),
                    targets
                        .into_iter()
                        .map(|target| PathBuf::from(relative_cache_path(repo_root, &target)))
                        .collect(),
                )
            })
            .collect(),
        nodes: graph
            .nodes
            .into_iter()
            .map(|node| PathBuf::from(relative_cache_path(repo_root, &node)))
            .collect(),
    }
}
