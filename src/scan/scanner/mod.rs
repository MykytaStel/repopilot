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
use crate::scan::config::ScanConfig;
use crate::scan::types::ScanSummary;
use std::io;
use std::path::Path;
use std::time::Instant;

pub use collection::{collect_scan_facts, collect_scan_facts_with_config};

pub fn scan_path(path: &Path) -> io::Result<ScanSummary> {
    scan_path_with_config(path, &ScanConfig::default())
}

pub fn scan_path_with_config(path: &Path, config: &ScanConfig) -> io::Result<ScanSummary> {
    let start = Instant::now();
    let file_audits = build_file_audits(config);
    let (mut facts, mut findings) =
        collection::collect_and_audit_inline(path, config, &file_audits)?;

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
    facts.react_native = react_native_profile.clone();

    findings.extend(run_project_audits(&facts, config));
    findings.extend(run_framework_audits(&facts, config));
    let (coupling_findings, coupling_graph) =
        ImportCouplingAudit.audit_with_graph(&facts, config, path);
    findings.extend(coupling_findings);

    for finding in &mut findings {
        finding.id = stable_finding_key(finding, path);
    }
    summary::sort_findings(&mut findings);

    let scan_duration_us = start.elapsed().as_micros() as u64;
    let health_score = ScanSummary::compute_health_score(&findings, facts.lines_of_code);

    Ok(ScanSummary {
        root_path: facts.root_path,
        files_discovered: facts.files_discovered,
        files_count: facts.files_count,
        directories_count: facts.directories_count,
        lines_of_code: facts.lines_of_code,
        skipped_files_count: facts.skipped_files_count,
        files_skipped_low_signal: facts.files_skipped_low_signal,
        binary_files_skipped: facts.binary_files_skipped,
        skipped_bytes: facts.skipped_bytes,
        languages: facts.languages,
        detected_frameworks: facts.detected_frameworks,
        framework_projects: facts.framework_projects,
        react_native: react_native_profile,
        findings,
        coupling_graph: Some(coupling_graph),
        scan_duration_us,
        health_score,
    })
}
