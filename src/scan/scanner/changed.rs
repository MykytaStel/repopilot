use super::changed_git::collect_changed_scope;
use crate::audits::architecture::import_coupling::ImportCouplingAudit;
use crate::audits::pipeline::{
    run_framework_audits, run_project_audits, stamp_findings_analysis_scope,
};
use crate::findings::aggregation::aggregate_duplicate_findings;
use crate::findings::enrichment::enrich_findings_timed;
use crate::findings::provenance::AnalysisScope;
use crate::findings::quality::summarize_signal_quality_with_contract_violations;
use crate::findings::rule_config::{apply_rule_config, rule_config_diagnostics};
use crate::knowledge::decision::apply_project_decisions;
use crate::review::diff::ChangedFile;
use crate::scan::config::ScanConfig;
use crate::scan::session::AnalysisSession;
use crate::scan::types::ScanSummary;
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

pub fn scan_changed_session(
    session: &AnalysisSession,
    base_ref: Option<&str>,
) -> io::Result<ScanSummary> {
    ChangedScanEngine::new(session.analysis_path(), session.scan_config(), base_ref).run()
}

pub fn scan_resolved_changed_with_config(
    path: &Path,
    config: &ScanConfig,
    repo_root: PathBuf,
    changed_files: Vec<ChangedFile>,
    base_ref: Option<&str>,
) -> io::Result<ScanSummary> {
    ChangedScanEngine::resolved(path, config, repo_root, changed_files, base_ref).run()
}

pub fn scan_resolved_changed_session(
    session: &AnalysisSession,
    changed_files: Vec<ChangedFile>,
    base_ref: Option<&str>,
) -> io::Result<ScanSummary> {
    ChangedScanEngine::resolved(
        session.analysis_path(),
        session.scan_config(),
        session.workspace_root().to_path_buf(),
        changed_files,
        base_ref,
    )
    .run()
}

struct ChangedScanEngine<'a> {
    path: &'a Path,
    config: &'a ScanConfig,
    base_ref: Option<&'a str>,
    resolved_scope: Option<ChangedDiscoveryStage>,
}

impl<'a> ChangedScanEngine<'a> {
    fn new(path: &'a Path, config: &'a ScanConfig, base_ref: Option<&'a str>) -> Self {
        Self {
            path,
            config,
            base_ref,
            resolved_scope: None,
        }
    }

    fn resolved(
        path: &'a Path,
        config: &'a ScanConfig,
        repo_root: PathBuf,
        changed_files: Vec<ChangedFile>,
        base_ref: Option<&'a str>,
    ) -> Self {
        Self {
            path,
            config,
            base_ref,
            resolved_scope: Some(ChangedDiscoveryStage {
                repo_root,
                changed_files,
                elapsed_us: 0,
            }),
        }
    }

    fn run(mut self) -> io::Result<ScanSummary> {
        let start = Instant::now();
        let discovery = match self.resolved_scope.take() {
            Some(discovery) => discovery,
            None => self.run_discovery()?,
        };
        if discovery.changed_files.is_empty() {
            return self.finalize_empty_changed(start, discovery);
        }
        let mut file_stage = self.run_file_analysis(&discovery)?;
        let repo_stage = self.run_repo_context(
            &discovery,
            &mut file_stage.facts,
            &file_stage.graph_patch_files,
            &mut file_stage.parsed_cache,
        )?;
        let project_start = Instant::now();
        let ((project_findings, framework_findings), (coupling_findings, _, _)) = rayon::join(
            || {
                rayon::join(
                    || run_project_audits(&repo_stage.repo_context, self.config),
                    || run_framework_audits(&repo_stage.repo_context, self.config),
                )
            },
            || {
                ImportCouplingAudit.audit_with_graph(
                    &repo_stage.repo_context,
                    self.config,
                    &discovery.repo_root,
                )
            },
        );
        let coupling_findings =
            stamp_findings_analysis_scope(coupling_findings, AnalysisScope::Repository);

        file_stage.findings.extend(project_findings);
        file_stage.findings.extend(framework_findings);
        file_stage.findings.extend(apply_project_decisions(
            &repo_stage.repo_context,
            coupling_findings,
        ));
        let post_scan_audits_us = project_start.elapsed().as_micros() as u64;
        let enrichment_us = enrich_findings_timed(&mut file_stage.findings, &discovery.repo_root);
        apply_rule_config(&mut file_stage.findings, self.config);
        aggregate_duplicate_findings(&mut file_stage.findings);
        let risk_scoring_us = self.score_findings(&repo_stage, &mut file_stage.findings);
        let contract_stage =
            super::contract_stage::validate_finding_contract_stage(&file_stage.findings);
        let signal_quality = summarize_signal_quality_with_contract_violations(
            &file_stage.findings,
            contract_stage.report.violations.len(),
        );
        let mut diagnostics = contract_stage.diagnostics;
        diagnostics.extend(rule_config_diagnostics(self.config));
        let finding_pipeline = ChangedFindingPipelineStage {
            post_scan_audits_us,
            enrichment_us,
            risk_scoring_us,
            contract_validation_us: contract_stage.elapsed_us,
            diagnostics,
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
mod stages;
use stages::*;
