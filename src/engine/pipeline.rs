use crate::engine::diagnostics::finding_contract_diagnostics;
use crate::findings::contract::validate_findings_contract;
use crate::findings::types::Finding;
use crate::scan::types::ScanDiagnostic;
use std::time::Instant;

pub fn validate_finding_contract_stage(
    findings: &[Finding],
    diagnostics: &mut Vec<ScanDiagnostic>,
) -> u64 {
    let start = Instant::now();
    let report = validate_findings_contract(findings);
    diagnostics.extend(finding_contract_diagnostics(&report));
    start.elapsed().as_micros() as u64
}
