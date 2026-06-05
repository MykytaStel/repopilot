use super::super::changed_telemetry::finalize_cache_telemetry;
use super::super::summary::{self, ScanSummaryParts, build_scan_summary};
use super::{
    ChangedDiscoveryStage, ChangedFileAnalysisStage, ChangedFindingPipelineStage,
    ChangedRepoContextStage, ChangedScanEngine,
};
use crate::findings::quality::SignalQualitySummary;
use crate::graph::context::summarize_context_graph;
use crate::scan::facts::ScanFacts;
use crate::scan::types::{ScanMode, ScanSummary, ScanTimings};
use std::io;
use std::time::Instant;

impl<'a> ChangedScanEngine<'a> {
    pub(super) fn finalize_report(
        &self,
        scan_start: Instant,
        discovery: ChangedDiscoveryStage,
        mut file_stage: ChangedFileAnalysisStage,
        repo_stage: ChangedRepoContextStage,
        finding_pipeline: ChangedFindingPipelineStage,
    ) -> io::Result<ScanSummary> {
        let finalization_start = Instant::now();

        summary::sort_findings(&mut file_stage.findings);
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
                    parse_us: file_stage.parse_us,
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

    pub(super) fn finalize_empty_changed(
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
                    parse_us: 0,
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
