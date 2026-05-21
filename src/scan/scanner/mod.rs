mod changed;
mod changed_cache;
mod changed_git;
mod changed_telemetry;
mod collection;
mod file;
mod summary;
mod walker;

use crate::audits::architecture::import_coupling::ImportCouplingAudit;
use crate::audits::pipeline::{build_file_audits, run_framework_audits, run_project_audits};
use crate::findings::enrichment::enrich_findings_timed;
use crate::findings::quality::summarize_signal_quality_with_contract_violations;
use crate::frameworks::{
    DetectedFramework, detect_framework_projects, detect_frameworks,
    detect_react_native_architecture,
};
use crate::knowledge::decision::apply_project_decisions;
use crate::risk::{apply_cluster_overlay, apply_graph_overlay, assess_findings};
use crate::scan::config::ScanConfig;
use crate::scan::types::{ScanMode, ScanSummary, ScanTimings};
use std::io;
use std::path::Path;
use std::time::Instant;
use summary::{ScanSummaryParts, build_scan_summary};

pub use changed::scan_changed_with_config;
pub use collection::{collect_scan_facts, collect_scan_facts_with_config};

pub fn scan_path(path: &Path) -> io::Result<ScanSummary> {
    scan_path_with_config(path, &ScanConfig::default())
}

pub fn scan_path_with_config(path: &Path, config: &ScanConfig) -> io::Result<ScanSummary> {
    ScanEngine::new(path, config).run()
}

pub struct ScanEngine<'a> {
    path: &'a Path,
    config: &'a ScanConfig,
}

struct FileAnalysisStage {
    facts: crate::scan::facts::ScanFacts,
    findings: Vec<crate::findings::types::Finding>,
    elapsed_us: u64,
}

struct DiscoveryStage {
    discovered: collection::DiscoveredScanPaths,
    elapsed_us: u64,
}

struct FrameworkDetectionStage {
    facts: crate::scan::facts::ScanFacts,
    elapsed_us: u64,
}

struct ProjectAnalysisStage {
    facts: crate::scan::facts::ScanFacts,
    findings: Vec<crate::findings::types::Finding>,
    coupling_graph: crate::graph::CouplingGraph,
    elapsed_us: u64,
}

impl<'a> ScanEngine<'a> {
    pub fn new(path: &'a Path, config: &'a ScanConfig) -> Self {
        Self { path, config }
    }

    pub fn run(self) -> io::Result<ScanSummary> {
        let start = Instant::now();
        let discovery_stage = self.run_discovery()?;
        let file_stage = self.run_file_analysis(discovery_stage.discovered)?;
        let framework_stage = self.run_framework_detection(file_stage.facts);
        let mut project_stage =
            self.run_project_analysis(framework_stage.facts, file_stage.findings);

        let enrichment_us = enrich_findings_timed(&mut project_stage.findings, self.path);
        let risk_scoring_us = self.score_findings(
            &project_stage.facts,
            &project_stage.coupling_graph,
            &mut project_stage.findings,
        );
        let contract_stage =
            crate::engine::pipeline::validate_finding_contract_stage(&project_stage.findings);
        let signal_quality = summarize_signal_quality_with_contract_violations(
            &project_stage.findings,
            contract_stage.report.violations.len(),
        );

        let scan_duration_us = start.elapsed().as_micros() as u64;
        let timings = ScanTimings {
            discovery_us: discovery_stage.elapsed_us,
            file_analysis_us: file_stage.elapsed_us,
            file_scan_us: discovery_stage
                .elapsed_us
                .saturating_add(file_stage.elapsed_us),
            framework_detection_us: framework_stage.elapsed_us,
            post_scan_audits_us: project_stage.elapsed_us,
            enrichment_us,
            risk_scoring_us,
            contract_validation_us: contract_stage.elapsed_us,
            report_finalization_us: 0,
        };

        Ok(self.finalize_report(
            project_stage,
            scan_duration_us,
            timings,
            contract_stage.diagnostics,
            signal_quality,
        ))
    }

    fn finalize_report(
        &self,
        mut project_stage: ProjectAnalysisStage,
        scan_duration_us: u64,
        timings: ScanTimings,
        diagnostics: Vec<crate::scan::types::ScanDiagnostic>,
        signal_quality: crate::findings::quality::SignalQualitySummary,
    ) -> ScanSummary {
        let finalization_start = Instant::now();
        summary::sort_findings(&mut project_stage.findings);
        let mut summary = build_scan_summary(
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
            },
        );
        if let Some(timings) = &mut summary.scan_timings {
            timings.report_finalization_us = finalization_start.elapsed().as_micros() as u64;
        }
        summary
    }

    fn run_discovery(&self) -> io::Result<DiscoveryStage> {
        let start = Instant::now();
        let discovered = collection::discover_scan_paths(self.path, self.config)?;
        Ok(DiscoveryStage {
            discovered,
            elapsed_us: start.elapsed().as_micros() as u64,
        })
    }

    fn run_file_analysis(
        &self,
        discovered: collection::DiscoveredScanPaths,
    ) -> io::Result<FileAnalysisStage> {
        let start = Instant::now();
        let file_audits = build_file_audits(self.config);
        let (facts, findings) =
            collection::analyze_discovered_files(discovered, &file_audits, self.config)?;
        Ok(FileAnalysisStage {
            facts,
            findings,
            elapsed_us: start.elapsed().as_micros() as u64,
        })
    }

    fn run_framework_detection(
        &self,
        mut facts: crate::scan::facts::ScanFacts,
    ) -> FrameworkDetectionStage {
        let start = Instant::now();
        facts.detected_frameworks = detect_frameworks(&facts.root_path);
        facts.framework_projects = detect_framework_projects(&facts.root_path);

        let react_native_profile = if facts
            .detected_frameworks
            .iter()
            .any(|f| matches!(f, DetectedFramework::ReactNative { .. }))
        {
            let profile = detect_react_native_architecture(&facts.root_path);
            if profile.detected {
                Some(profile)
            } else {
                None
            }
        } else {
            None
        };
        facts.react_native = react_native_profile;

        FrameworkDetectionStage {
            facts,
            elapsed_us: start.elapsed().as_micros() as u64,
        }
    }

    fn run_project_analysis(
        &self,
        facts: crate::scan::facts::ScanFacts,
        mut findings: Vec<crate::findings::types::Finding>,
    ) -> ProjectAnalysisStage {
        let start = Instant::now();
        let ((project_findings, framework_findings), (coupling_findings, coupling_graph)) =
            rayon::join(
                || {
                    rayon::join(
                        || run_project_audits(&facts, self.config),
                        || run_framework_audits(&facts, self.config),
                    )
                },
                || ImportCouplingAudit.audit_with_graph(&facts, self.config, self.path),
            );
        findings.extend(project_findings);
        findings.extend(framework_findings);
        findings.extend(apply_project_decisions(&facts, coupling_findings));

        ProjectAnalysisStage {
            facts,
            findings,
            coupling_graph,
            elapsed_us: start.elapsed().as_micros() as u64,
        }
    }

    fn score_findings(
        &self,
        facts: &crate::scan::facts::ScanFacts,
        coupling_graph: &crate::graph::CouplingGraph,
        findings: &mut [crate::findings::types::Finding],
    ) -> u64 {
        let start = Instant::now();
        assess_findings(findings, facts);
        // Graph and cluster overlays are kept after the base scoring pass because
        // they depend on the complete cross-file project view.
        apply_graph_overlay(findings, coupling_graph);
        apply_cluster_overlay(findings);
        start.elapsed().as_micros() as u64
    }
}
