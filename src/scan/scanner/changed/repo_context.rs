use super::super::collection;
use super::{ChangedDiscoveryStage, ChangedRepoContextStage, ChangedScanEngine};
use crate::findings::types::Finding;
use crate::frameworks::{
    DetectedFramework, detect_framework_projects, detect_frameworks,
    detect_react_native_architecture,
};
use crate::graph::context::{
    RepositoryContextState, RepositoryContextStateLoad, context_graph_cache_miss,
    load_repository_context_state, write_repository_context_state,
};
use crate::graph::{CouplingGraph, build_coupling_graph};
use crate::risk::{apply_cluster_overlay, apply_graph_overlay, assess_findings};
use crate::scan::cache::{config_fingerprint, relative_cache_path};
use crate::scan::facts::{FileFacts, ScanFacts};
use crate::scan::parsed_cache::ParsedFactsCache;
use crate::scan::types::cache_diagnostic;
use repo_context_facts::{absolutize_scan_fact_paths, apply_changed_context_facts};
use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

#[path = "repo_context_facts.rs"]
mod repo_context_facts;

impl<'a> ChangedScanEngine<'a> {
    pub(super) fn run_repo_context(
        &self,
        discovery: &ChangedDiscoveryStage,
        facts: &mut ScanFacts,
        graph_patch_files: &[FileFacts],
        parsed_cache: &mut ParsedFactsCache,
    ) -> io::Result<ChangedRepoContextStage> {
        let start = Instant::now();
        let repo_root = &discovery.repo_root;
        let fingerprint = config_fingerprint(self.config);

        if let Some(load) = load_repository_context_state(repo_root, &fingerprint) {
            return Ok(self.cached_repo_context(
                discovery,
                facts,
                graph_patch_files,
                &fingerprint,
                start,
                load,
            ));
        }

        self.cold_repo_context(
            discovery,
            facts,
            graph_patch_files,
            parsed_cache,
            start,
            fingerprint,
        )
    }

    fn cached_repo_context(
        &self,
        discovery: &ChangedDiscoveryStage,
        facts: &mut ScanFacts,
        graph_patch_files: &[FileFacts],
        fingerprint: &str,
        start: Instant,
        mut load: RepositoryContextStateLoad,
    ) -> ChangedRepoContextStage {
        let repo_root = &discovery.repo_root;
        load.state.apply_changed_facts_with_spans(
            repo_root,
            &discovery.changed_files,
            graph_patch_files,
            &facts.import_spans_by_file,
            &facts.parsed_content_hashes,
            &facts.guarded_optional_imports_by_file,
        );
        let diagnostics = write_repository_context_state(repo_root, fingerprint, &load.state)
            .err()
            .map(|error| vec![cache_diagnostic(&error)])
            .unwrap_or_default();
        cached_stage(load, repo_root, facts, diagnostics, start)
    }

    #[allow(clippy::too_many_arguments)]
    fn cold_repo_context(
        &self,
        discovery: &ChangedDiscoveryStage,
        facts: &mut ScanFacts,
        graph_patch_files: &[FileFacts],
        parsed_cache: &mut ParsedFactsCache,
        start: Instant,
        fingerprint: String,
    ) -> io::Result<ChangedRepoContextStage> {
        let repo_root = &discovery.repo_root;
        let mut repo_context = collection::collect_scan_facts_without_content_with_parsed_cache(
            repo_root,
            self.config,
            parsed_cache,
        )?;
        parsed_cache.retain_referenced_current_scan();
        detect_repo_frameworks(&mut repo_context, repo_root);

        let coupling_graph =
            relative_coupling_graph(build_coupling_graph(&repo_context, repo_root), repo_root);
        let mut context_state =
            RepositoryContextState::from_scan_facts(&repo_context, repo_root, coupling_graph);
        apply_stage_changes(&mut context_state, discovery, facts, graph_patch_files);
        let coupling_graph = context_state.coupling_graph();
        let (cache_info, diagnostics) =
            persist_cold_context(repo_root, &fingerprint, &context_state);
        prepare_cold_audit_facts(
            &mut repo_context,
            discovery,
            facts,
            graph_patch_files,
            repo_root,
        );

        Ok(ChangedRepoContextStage {
            repo_context,
            coupling_graph,
            context_state,
            cache_info,
            diagnostics,
            elapsed_us: start.elapsed().as_micros() as u64,
        })
    }

    pub(super) fn score_findings(
        &self,
        repo_stage: &ChangedRepoContextStage,
        findings: &mut [Finding],
    ) -> u64 {
        let start = Instant::now();
        assess_findings(findings, &repo_stage.repo_context);
        apply_graph_overlay(findings, &repo_stage.coupling_graph);
        apply_cluster_overlay(findings);
        start.elapsed().as_micros() as u64
    }
}

fn detect_repo_frameworks(repo_context: &mut ScanFacts, repo_root: &Path) {
    repo_context.detected_frameworks = detect_frameworks(repo_root);
    repo_context.framework_projects = detect_framework_projects(repo_root);
    repo_context.react_native = detect_react_native_profile(repo_context);
}

fn apply_stage_changes(
    context_state: &mut RepositoryContextState,
    discovery: &ChangedDiscoveryStage,
    facts: &ScanFacts,
    graph_patch_files: &[FileFacts],
) {
    context_state.apply_changed_facts_with_spans(
        &discovery.repo_root,
        &discovery.changed_files,
        graph_patch_files,
        &facts.import_spans_by_file,
        &facts.parsed_content_hashes,
        &facts.guarded_optional_imports_by_file,
    );
}

fn persist_cold_context(
    repo_root: &Path,
    fingerprint: &str,
    context_state: &RepositoryContextState,
) -> (
    crate::scan::types::ContextGraphCacheInfo,
    Vec<crate::scan::types::ScanDiagnostic>,
) {
    let mut cache_info =
        context_graph_cache_miss(repo_root, "missing-or-invalid-context-graph-cache");
    let mut diagnostics = Vec::new();
    match write_repository_context_state(repo_root, fingerprint, context_state) {
        Ok(_) => cache_info.reason.push_str("; cache-updated"),
        Err(error) => diagnostics.push(cache_diagnostic(&error)),
    }
    (cache_info, diagnostics)
}

fn prepare_cold_audit_facts(
    repo_context: &mut ScanFacts,
    discovery: &ChangedDiscoveryStage,
    facts: &mut ScanFacts,
    graph_patch_files: &[FileFacts],
    repo_root: &Path,
) {
    // Keep the full cold-scan facts for audits; the persisted state is summary-only.
    apply_changed_context_facts(
        repo_context,
        repo_root,
        &discovery.changed_files,
        graph_patch_files,
        &facts.import_spans_by_file,
        &facts.parsed_content_hashes,
        &facts.guarded_optional_imports_by_file,
    );
    absolutize_scan_fact_paths(repo_context, repo_root);
    copy_framework_facts(repo_context, facts);
}

fn copy_framework_facts(repo_context: &ScanFacts, facts: &mut ScanFacts) {
    facts.detected_frameworks = repo_context.detected_frameworks.clone();
    facts.framework_projects = repo_context.framework_projects.clone();
    facts.react_native = repo_context.react_native.clone();
}

fn cached_stage(
    load: RepositoryContextStateLoad,
    repo_root: &Path,
    facts: &mut ScanFacts,
    diagnostics: Vec<crate::scan::types::ScanDiagnostic>,
    start: Instant,
) -> ChangedRepoContextStage {
    let mut repo_context = load.state.to_scan_facts();
    absolutize_scan_fact_paths(&mut repo_context, repo_root);
    let coupling_graph = load.state.coupling_graph();
    copy_framework_facts(&repo_context, facts);
    ChangedRepoContextStage {
        repo_context,
        coupling_graph,
        context_state: load.state,
        cache_info: load.cache_info,
        diagnostics,
        elapsed_us: start.elapsed().as_micros() as u64,
    }
}

fn detect_react_native_profile(
    facts: &ScanFacts,
) -> Option<crate::frameworks::ReactNativeArchitectureProfile> {
    if facts
        .detected_frameworks
        .iter()
        .any(|framework| matches!(framework, DetectedFramework::ReactNative { .. }))
    {
        let profile = detect_react_native_architecture(&facts.root_path);
        if profile.detected {
            return Some(profile);
        }
    }
    None
}

fn relative_coupling_graph(graph: CouplingGraph, repo_root: &Path) -> CouplingGraph {
    let relativize_edges = |edges: BTreeMap<PathBuf, BTreeSet<PathBuf>>| {
        edges
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
            .collect()
    };

    CouplingGraph {
        edges: relativize_edges(graph.edges),
        deferred_edges: relativize_edges(graph.deferred_edges),
        nodes: graph
            .nodes
            .into_iter()
            .map(|node| PathBuf::from(relative_cache_path(repo_root, &node)))
            .collect(),
    }
}

#[cfg(test)]
#[path = "repo_context_tests.rs"]
mod tests;
