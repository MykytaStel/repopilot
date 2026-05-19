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
use crate::baseline::key::stable_finding_key;
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
        let file_stage = self.run_file_analysis()?;
        let framework_stage = self.run_framework_detection(file_stage.facts);
        let mut project_stage =
            self.run_project_analysis(framework_stage.facts, file_stage.findings);

        self.enrich_findings(
            &project_stage.facts,
            &project_stage.coupling_graph,
            &mut project_stage.findings,
        );

        let scan_duration_us = start.elapsed().as_micros() as u64;
        Ok(build_scan_summary(
            project_stage.facts,
            project_stage.findings,
            ScanSummaryParts {
                mode: ScanMode::Full,
                base_ref: None,
                changed_files_count: 0,
                repo_level_rules_included: true,
                coupling_graph: Some(project_stage.coupling_graph),
                scan_duration_us,
                scan_timings: Some(ScanTimings {
                    file_scan_us: file_stage.elapsed_us,
                    framework_detection_us: framework_stage.elapsed_us,
                    post_scan_audits_us: project_stage.elapsed_us,
                }),
                cache_telemetry: None,
                diagnostics: Vec::new(),
            },
        ))
    }

    fn run_file_analysis(&self) -> io::Result<FileAnalysisStage> {
        let start = Instant::now();
        let file_audits = build_file_audits(self.config);
        let (facts, findings) =
            collection::collect_and_audit_inline(self.path, self.config, &file_audits)?;
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

    fn enrich_findings(
        &self,
        facts: &crate::scan::facts::ScanFacts,
        coupling_graph: &crate::graph::CouplingGraph,
        findings: &mut [crate::findings::types::Finding],
    ) {
        for finding in findings.iter_mut() {
            finding.populate_recommendation();
            finding.id = stable_finding_key(finding, self.path);
        }
        assess_findings(findings, facts);
        // Graph and cluster overlays are kept after the base scoring pass because
        // they depend on the complete cross-file project view.
        apply_graph_overlay(findings, coupling_graph);
        apply_cluster_overlay(findings);
        summary::sort_findings(findings);
    }
}
