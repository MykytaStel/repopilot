use crate::audits::architecture::large_file::LargeFileAudit;
use crate::audits::code_quality::code_markers::CodeMarkerAudit;
use crate::audits::traits::{FileAudit, ProjectAudit};
use crate::findings::types::Finding;
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;

pub fn run_audits(scan_facts: &ScanFacts, config: &ScanConfig) -> Vec<Finding> {
    let file_audits: Vec<Box<dyn FileAudit>> =
        vec![Box::new(LargeFileAudit), Box::new(CodeMarkerAudit)];

    let project_audits: Vec<Box<dyn ProjectAudit>> = vec![];

    let mut findings: Vec<Finding> = scan_facts
        .files
        .iter()
        .flat_map(|file| file_audits.iter().flat_map(|a| a.audit(file, config)))
        .collect();

    findings.extend(
        project_audits
            .iter()
            .flat_map(|a| a.audit(scan_facts, config)),
    );

    findings
}
