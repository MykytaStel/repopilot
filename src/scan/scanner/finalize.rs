use super::summary::{ScanSummaryParts, build_scan_summary};
use super::{full::ProjectAnalysisStage, full::ScanEngine, summary};
use crate::findings::quality::SignalQualitySummary;
use crate::graph::context::{
    ContextGraphCacheInfo, ContextGraphSummary, RepositoryContextState,
    summarize_repository_context_state, write_repository_context_state,
};
use crate::scan::cache::config_fingerprint;
use crate::scan::config::ScanConfig;
use crate::scan::types::{ScanDiagnostic, ScanMode, ScanSummary, ScanTimings, cache_diagnostic};
use std::path::Path;
use std::time::Instant;

struct ContextGraphArtifacts {
    summary: ContextGraphSummary,
    cache: Option<ContextGraphCacheInfo>,
}

impl<'a> ScanEngine<'a> {
    pub(super) fn finalize_report(
        &self,
        mut project_stage: ProjectAnalysisStage,
        scan_duration_us: u64,
        timings: ScanTimings,
        diagnostics: Vec<ScanDiagnostic>,
        signal_quality: SignalQualitySummary,
    ) -> ScanSummary {
        let finalization_start = Instant::now();
        let context_state = prepare_findings_and_context_state(&mut project_stage);
        let mut diagnostics = diagnostics;
        if crate::graph::was_cycle_detection_depth_exceeded() {
            diagnostics.push(ScanDiagnostic::warning(
                "graph.cycle-depth-exceeded",
                "Cycle detection depth limit (512 hops) was exceeded; some deep transitive cycles may not have been reported.",
            ));
        }
        let context_graph_artifacts = build_context_graph_artifacts(
            &project_stage.facts.root_path,
            self.config,
            &context_state,
            &project_stage.findings,
            &mut diagnostics,
        );

        let mut summary = build_final_scan_summary(
            project_stage,
            scan_duration_us,
            timings,
            diagnostics,
            signal_quality,
            context_graph_artifacts,
        );

        record_report_finalization_timing(&mut summary, finalization_start);
        summary
    }
}

fn prepare_findings_and_context_state(
    project_stage: &mut ProjectAnalysisStage,
) -> RepositoryContextState {
    summary::sort_findings(&mut project_stage.findings);

    RepositoryContextState::from_scan_facts(
        &project_stage.facts,
        &project_stage.facts.root_path,
        project_stage.coupling_graph.clone(),
    )
}

fn build_context_graph_artifacts(
    root_path: &Path,
    config: &ScanConfig,
    context_state: &RepositoryContextState,
    findings: &[crate::findings::types::Finding],
    diagnostics: &mut Vec<ScanDiagnostic>,
) -> ContextGraphArtifacts {
    let graph_summary = summarize_repository_context_state(context_state, findings, &[]);
    let fingerprint = config_fingerprint(config);
    let cache = match write_repository_context_state(root_path, &fingerprint, context_state) {
        Ok(cache_info) => Some(cache_info),
        Err(error) => {
            diagnostics.push(cache_diagnostic(&error));
            None
        }
    };

    ContextGraphArtifacts {
        summary: graph_summary,
        cache,
    }
}

fn build_final_scan_summary(
    project_stage: ProjectAnalysisStage,
    scan_duration_us: u64,
    timings: ScanTimings,
    diagnostics: Vec<ScanDiagnostic>,
    signal_quality: SignalQualitySummary,
    context_graph_artifacts: ContextGraphArtifacts,
) -> ScanSummary {
    build_scan_summary(
        project_stage.facts,
        project_stage.findings,
        ScanSummaryParts {
            mode: ScanMode::Full,
            base_ref: None,
            changed_files_count: 0,
            repo_level_rules_included: true,
            coupling_graph: Some(project_stage.coupling_graph),
            scan_duration_us,
            scan_timings: Some(timings),
            cache_telemetry: None,
            diagnostics,
            signal_quality,
            context_graph_summary: Some(context_graph_artifacts.summary),
            context_graph_cache: context_graph_artifacts.cache,
        },
    )
}

fn record_report_finalization_timing(summary: &mut ScanSummary, finalization_start: Instant) {
    if let Some(timings) = &mut summary.scan_timings {
        timings.report_finalization_us = finalization_start.elapsed().as_micros() as u64;
    }
}
