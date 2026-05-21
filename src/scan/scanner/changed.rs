use super::changed_cache::{
    CacheDecision, cache_decision, normalize_per_file_paths, record_cached_file,
};
use super::changed_git::collect_changed_scope;
use super::changed_telemetry::{
    change_status_label, finalize_cache_telemetry, record_skipped_cache_file,
};
use super::file::{SkipReason, process_file_with_content};
use super::summary::{ScanSummaryParts, build_language_summary, build_scan_summary};
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
    ContextGraphCacheInfo, RepoContextGraph, cache_diagnostic, context_graph_cache_miss,
    load_repo_context_graph, summarize_context_graph, write_repo_context_graph,
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
            crate::engine::pipeline::validate_finding_contract_stage(&file_stage.findings);
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

    fn run_file_analysis(
        &self,
        discovery: &ChangedDiscoveryStage,
    ) -> io::Result<ChangedFileAnalysisStage> {
        let start = Instant::now();
        let file_audits = build_file_audits(self.config);
        let cache_load_start = Instant::now();
        let mut cache = ScanCache::load(&discovery.repo_root);
        let mut cache_telemetry = ScanCacheTelemetry {
            timings: crate::scan::types::ScanCacheTimings {
                load_us: cache_load_start.elapsed().as_micros() as u64,
                ..Default::default()
            },
            ..Default::default()
        };

        let fingerprint = config_fingerprint(self.config);
        let mut facts = ScanFacts {
            root_path: self.path.to_path_buf(),
            files_discovered: discovery.changed_files.len(),
            ..ScanFacts::default()
        };
        let mut languages: HashMap<String, usize> = HashMap::new();
        let mut findings = Vec::new();
        let mut graph_patch_files = Vec::new();
        let mut directories = HashSet::new();
        let mut changed_file_reasons: BTreeMap<String, usize> = BTreeMap::new();

        for changed_file in &discovery.changed_files {
            self.process_changed_file(
                changed_file,
                &discovery.repo_root,
                &fingerprint,
                &file_audits,
                &mut facts,
                &mut languages,
                &mut findings,
                &mut directories,
                &mut cache,
                &mut cache_telemetry,
                &mut changed_file_reasons,
                &mut graph_patch_files,
            )?;
        }

        facts.directories_count = directories.len();
        facts.languages = build_language_summary(languages);

        Ok(ChangedFileAnalysisStage {
            facts,
            findings,
            graph_patch_files,
            cache,
            cache_telemetry,
            changed_file_reasons,
            elapsed_us: start.elapsed().as_micros() as u64,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn process_changed_file(
        &self,
        changed_file: &ChangedFile,
        repo_root: &Path,
        fingerprint: &str,
        file_audits: &[Box<dyn crate::audits::traits::FileAudit>],
        facts: &mut ScanFacts,
        languages: &mut HashMap<String, usize>,
        findings: &mut Vec<Finding>,
        directories: &mut HashSet<PathBuf>,
        cache: &mut ScanCache,
        cache_telemetry: &mut ScanCacheTelemetry,
        changed_file_reasons: &mut BTreeMap<String, usize>,
        graph_patch_files: &mut Vec<FileFacts>,
    ) -> io::Result<()> {
        let change_reason = change_status_label(changed_file.status).to_string();
        *changed_file_reasons
            .entry(change_reason.clone())
            .or_insert(0) += 1;

        if changed_file.status == ChangeStatus::Deleted {
            record_skipped_cache_file(
                cache_telemetry,
                &changed_file.path,
                &change_reason,
                "deleted",
            );
            return Ok(());
        }

        let absolute_path = repo_root.join(&changed_file.path);
        if !absolute_path.is_file() {
            let reason = if absolute_path.exists() {
                "not-regular-file"
            } else {
                "missing-file"
            };
            record_skipped_cache_file(cache_telemetry, &changed_file.path, &change_reason, reason);
            return Ok(());
        }

        if let Some(parent) = changed_file.path.parent()
            && !parent.as_os_str().is_empty()
        {
            directories.insert(parent.to_path_buf());
        }

        let hash_start = Instant::now();
        let hash_entry = file_hash_entry(repo_root, &absolute_path)?;
        cache_telemetry.timings.file_hash_us = cache_telemetry
            .timings
            .file_hash_us
            .saturating_add(hash_start.elapsed().as_micros() as u64);

        let cache_path = hash_entry.path.clone();
        cache
            .file_hashes
            .insert(cache_path.clone(), hash_entry.clone());

        let lookup_start = Instant::now();
        let decision = cache_decision(
            cache.file_roles.get(&cache_path),
            cache.findings.get(&cache_path),
            &hash_entry.hash,
            fingerprint,
        );
        cache_telemetry.timings.lookup_us = cache_telemetry
            .timings
            .lookup_us
            .saturating_add(lookup_start.elapsed().as_micros() as u64);

        let mut cache_reason = match decision {
            CacheDecision::Hit {
                role_entry,
                findings: cached_findings,
            } => {
                self.reuse_cached_file(
                    changed_file,
                    &change_reason,
                    facts,
                    languages,
                    findings,
                    cache_telemetry,
                    role_entry,
                    cached_findings,
                );
                return Ok(());
            }
            CacheDecision::Miss { reason } => reason,
        };

        let miss_start = Instant::now();
        let mut per_file = process_file_with_content(&absolute_path, file_audits, self.config)?;
        normalize_per_file_paths(
            &mut per_file.file_facts.path,
            &mut per_file.findings,
            repo_root,
        );

        match per_file.skip_reason {
            SkipReason::None => {
                facts.files_analyzed += 1;
                facts.non_empty_lines += per_file.file_facts.non_empty_lines;
                if let Some(language) = &per_file.language {
                    *languages.entry(language.clone()).or_insert(0) += 1;
                }

                if let Some(context) = per_file.context {
                    cache.file_roles.insert(
                        cache_path.clone(),
                        FileRoleEntry {
                            path: cache_path.clone(),
                            hash: hash_entry.hash.clone(),
                            language: per_file.language.clone(),
                            non_empty_lines: per_file.file_facts.non_empty_lines,
                            roles: context.roles,
                            frameworks: context.frameworks,
                            runtimes: context.runtimes,
                            paradigms: context.paradigms,
                            is_test: context.is_test,
                        },
                    );
                }

                cache.findings.insert(
                    cache_path,
                    FindingsEntry {
                        path: hash_entry.path,
                        hash: hash_entry.hash,
                        config_fingerprint: fingerprint.to_string(),
                        findings: per_file.findings.clone(),
                    },
                );

                let mut graph_file = per_file.file_facts.clone();
                graph_file.content = None;
                graph_patch_files.push(graph_file);

                facts.files.push(per_file.file_facts);
                findings.extend(per_file.findings);
            }
            SkipReason::LargeFile => {
                cache_reason = "large-file";
                facts.large_files_skipped += 1;
                facts.skipped_bytes = facts.skipped_bytes.saturating_add(per_file.skipped_bytes);
            }
            SkipReason::Binary => {
                cache_reason = "binary-file";
                facts.binary_files_skipped += 1;
                facts.skipped_bytes = facts.skipped_bytes.saturating_add(per_file.skipped_bytes);
            }
            SkipReason::LowSignal => {
                cache_reason = "low-signal-file";
                facts.files_skipped_low_signal += 1;
            }
        }

        cache_telemetry.timings.miss_scan_us = cache_telemetry
            .timings
            .miss_scan_us
            .saturating_add(miss_start.elapsed().as_micros() as u64);
        cache_telemetry.misses += 1;
        cache_telemetry
            .changed_files
            .push(ChangedFileCacheTelemetry {
                path: changed_file.path.clone(),
                change_reason,
                cache_status: "miss".to_string(),
                cache_reason: cache_reason.to_string(),
            });

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn reuse_cached_file(
        &self,
        changed_file: &ChangedFile,
        change_reason: &str,
        facts: &mut ScanFacts,
        languages: &mut HashMap<String, usize>,
        findings: &mut Vec<Finding>,
        cache_telemetry: &mut ScanCacheTelemetry,
        role_entry: FileRoleEntry,
        cached_findings: Vec<Finding>,
    ) {
        let reuse_start = Instant::now();
        record_cached_file(facts, languages, &role_entry);
        findings.extend(cached_findings);
        cache_telemetry.timings.hit_reuse_us = cache_telemetry
            .timings
            .hit_reuse_us
            .saturating_add(reuse_start.elapsed().as_micros() as u64);
        cache_telemetry.hits += 1;
        cache_telemetry
            .changed_files
            .push(ChangedFileCacheTelemetry {
                path: changed_file.path.clone(),
                change_reason: change_reason.to_string(),
                cache_status: "hit".to_string(),
                cache_reason: "unchanged-content-and-config".to_string(),
            });
    }

    fn run_repo_context(
        &self,
        discovery: &ChangedDiscoveryStage,
        facts: &mut ScanFacts,
        graph_patch_files: &[FileFacts],
    ) -> io::Result<ChangedRepoContextStage> {
        let start = Instant::now();
        let mut diagnostics = Vec::new();
        let repo_root = &discovery.repo_root;

        if let Some(mut load) = load_repo_context_graph(repo_root, self.config) {
            load.graph
                .apply_changed_facts(repo_root, &discovery.changed_files, graph_patch_files);
            if let Err(error) = write_repo_context_graph(repo_root, self.config, &load.graph) {
                diagnostics.push(cache_diagnostic(&error));
            }

            let repo_context = load.graph.to_scan_facts();
            let coupling_graph = load.graph.coupling_graph();

            facts.detected_frameworks = repo_context.detected_frameworks.clone();
            facts.framework_projects = repo_context.framework_projects.clone();
            facts.react_native = repo_context.react_native.clone();

            return Ok(ChangedRepoContextStage {
                repo_context,
                coupling_graph,
                context_graph: load.graph,
                cache_info: load.cache_info,
                diagnostics,
                elapsed_us: start.elapsed().as_micros() as u64,
            });
        }

        let mut repo_context =
            super::collection::collect_scan_facts_without_content(repo_root, self.config)?;

        repo_context.detected_frameworks = detect_frameworks(repo_root);
        repo_context.framework_projects = detect_framework_projects(repo_root);
        repo_context.react_native = detect_react_native_profile(&repo_context);

        let coupling_graph =
            relative_coupling_graph(build_coupling_graph(&repo_context, repo_root), repo_root);
        let context_graph =
            RepoContextGraph::from_scan_facts(&repo_context, repo_root, coupling_graph.clone());
        let mut cache_info =
            context_graph_cache_miss(repo_root, "missing-or-invalid-context-graph-cache");
        match write_repo_context_graph(repo_root, self.config, &context_graph) {
            Ok(_) => {
                cache_info.reason.push_str("; cache-updated");
            }
            Err(error) => diagnostics.push(cache_diagnostic(&error)),
        }

        facts.detected_frameworks = repo_context.detected_frameworks.clone();
        facts.framework_projects = repo_context.framework_projects.clone();
        facts.react_native = repo_context.react_native.clone();

        Ok(ChangedRepoContextStage {
            repo_context,
            coupling_graph,
            context_graph,
            cache_info,
            diagnostics,
            elapsed_us: start.elapsed().as_micros() as u64,
        })
    }

    fn score_findings(
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

    fn finalize_report(
        &self,
        scan_start: Instant,
        discovery: ChangedDiscoveryStage,
        mut file_stage: ChangedFileAnalysisStage,
        repo_stage: ChangedRepoContextStage,
        finding_pipeline: ChangedFindingPipelineStage,
    ) -> io::Result<ScanSummary> {
        let finalization_start = Instant::now();

        super::summary::sort_findings(&mut file_stage.findings);
        let context_graph_summary = summarize_context_graph(
            &repo_stage.context_graph,
            &file_stage.findings,
            &discovery.changed_files,
        );
        let mut diagnostics = repo_stage.diagnostics;
        diagnostics.extend(finding_pipeline.diagnostics);

        let cache_write_start = Instant::now();
        file_stage.cache.write(&discovery.repo_root)?;
        file_stage.cache_telemetry.timings.write_us =
            cache_write_start.elapsed().as_micros() as u64;
        finalize_cache_telemetry(
            &mut file_stage.cache_telemetry,
            file_stage.changed_file_reasons,
        );

        file_stage.facts.root_path = self.path.to_path_buf();

        let scan_duration_us = scan_start.elapsed().as_micros() as u64;
        let file_scan_us = discovery.elapsed_us.saturating_add(file_stage.elapsed_us);

        let mut summary = build_scan_summary(
            file_stage.facts,
            file_stage.findings,
            ScanSummaryParts {
                mode: ScanMode::Changed,
                base_ref: self.base_ref.map(str::to_string),
                changed_files_count: discovery.changed_files.len(),
                repo_level_rules_included: false,
                coupling_graph: None,
                scan_duration_us,
                scan_timings: Some(ScanTimings {
                    discovery_us: discovery.elapsed_us,
                    file_analysis_us: file_stage.elapsed_us,
                    file_scan_us,
                    framework_detection_us: repo_stage.elapsed_us,
                    post_scan_audits_us: 0,
                    enrichment_us: finding_pipeline.enrichment_us,
                    risk_scoring_us: finding_pipeline.risk_scoring_us,
                    contract_validation_us: finding_pipeline.contract_validation_us,
                    report_finalization_us: 0,
                }),
                cache_telemetry: Some(file_stage.cache_telemetry),
                context_graph_summary: Some(context_graph_summary),
                context_graph_cache: Some(repo_stage.cache_info),
                diagnostics,
                signal_quality: finding_pipeline.signal_quality,
            },
        );

        if let Some(timings) = &mut summary.scan_timings {
            timings.report_finalization_us = finalization_start.elapsed().as_micros() as u64;
        }

        Ok(summary)
    }

    fn finalize_empty_changed(
        &self,
        scan_start: Instant,
        discovery: ChangedDiscoveryStage,
    ) -> io::Result<ScanSummary> {
        let finalization_start = Instant::now();
        let scan_duration_us = scan_start.elapsed().as_micros() as u64;
        let mut summary = build_scan_summary(
            ScanFacts {
                root_path: self.path.to_path_buf(),
                ..ScanFacts::default()
            },
            Vec::new(),
            ScanSummaryParts {
                mode: ScanMode::Changed,
                base_ref: self.base_ref.map(str::to_string),
                changed_files_count: 0,
                repo_level_rules_included: false,
                coupling_graph: None,
                scan_duration_us,
                scan_timings: Some(ScanTimings {
                    discovery_us: discovery.elapsed_us,
                    file_analysis_us: 0,
                    file_scan_us: discovery.elapsed_us,
                    framework_detection_us: 0,
                    post_scan_audits_us: 0,
                    enrichment_us: 0,
                    risk_scoring_us: 0,
                    contract_validation_us: 0,
                    report_finalization_us: 0,
                }),
                cache_telemetry: None,
                context_graph_summary: None,
                context_graph_cache: None,
                diagnostics: Vec::new(),
                signal_quality: SignalQualitySummary::default(),
            },
        );
        if let Some(timings) = &mut summary.scan_timings {
            timings.report_finalization_us = finalization_start.elapsed().as_micros() as u64;
        }
        Ok(summary)
    }
}

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
