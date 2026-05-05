use crate::audits::architecture::large_file::detect_large_file_finding;
use crate::audits::code_quality::code_markers::detect_code_marker_findings;
use crate::findings::types::Finding;
use crate::scan::config::ScanConfig;
use crate::scan::facts::{FileFacts, ScanFacts};

pub fn run_audits(scan_facts: &ScanFacts, config: &ScanConfig) -> Vec<Finding> {
    let mut findings = Vec::new();

    for file in &scan_facts.files {
        findings.extend(run_file_audits(file, config));
    }

    findings
}

fn run_file_audits(file: &FileFacts, config: &ScanConfig) -> Vec<Finding> {
    let mut findings = Vec::new();

    if let Some(finding) = detect_large_file_finding(&file.path, file.lines_of_code, config) {
        findings.push(finding);
    }

    findings.extend(detect_code_marker_findings(&file.path, &file.content));

    findings
}
