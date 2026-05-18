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
    let start = Instant::now();
    let file_audits = build_file_audits(config);
    let (mut facts, mut findings) =
        collection::collect_and_audit_inline(path, config, &file_audits)?;
    let file_scan_us = start.elapsed().as_micros() as u64;

    let framework_start = Instant::now();
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
    let framework_detection_us = framework_start.elapsed().as_micros() as u64;

    let post_scan_start = Instant::now();
    let ((project_findings, framework_findings), (coupling_findings, coupling_graph)) = rayon::join(
        || {
            rayon::join(
                || run_project_audits(&facts, config),
                || run_framework_audits(&facts, config),
            )
        },
        || ImportCouplingAudit.audit_with_graph(&facts, config, path),
    );
    findings.extend(project_findings);
    findings.extend(framework_findings);
    findings.extend(apply_project_decisions(&facts, coupling_findings));
    let post_scan_audits_us = post_scan_start.elapsed().as_micros() as u64;

    for finding in &mut findings {
        finding.populate_recommendation();
        finding.id = stable_finding_key(finding, path);
    }
    assess_findings(&mut findings, &facts);
    apply_graph_overlay(&mut findings, &coupling_graph);
    apply_cluster_overlay(&mut findings);
    summary::sort_findings(&mut findings);

    let scan_duration_us = start.elapsed().as_micros() as u64;
    Ok(build_scan_summary(
        facts,
        findings,
        ScanSummaryParts {
            mode: ScanMode::Full,
            base_ref: None,
            changed_files_count: 0,
            repo_level_rules_included: true,
            coupling_graph: Some(coupling_graph),
            scan_duration_us,
            scan_timings: Some(ScanTimings {
                file_scan_us,
                framework_detection_us,
                post_scan_audits_us,
            }),
            cache_telemetry: None,
        },
    ))
}
