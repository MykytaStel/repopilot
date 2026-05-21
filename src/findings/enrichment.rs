use crate::baseline::key::stable_finding_key;
use crate::findings::types::Finding;
use std::path::Path;
use std::time::Instant;

pub fn enrich_findings(findings: &mut [Finding], root: &Path) {
    for finding in findings {
        enrich_finding(finding, root);
    }
}

pub fn enrich_findings_timed(findings: &mut [Finding], root: &Path) -> u64 {
    let start = Instant::now();
    enrich_findings(findings, root);
    start.elapsed().as_micros() as u64
}

pub fn enrich_finding(finding: &mut Finding, root: &Path) {
    finding.populate_rule_metadata();
    finding.id = stable_finding_key(finding, root);
}
