use crate::engine::diagnostics::finding_contract_diagnostics;
use crate::findings::contract::{FindingContractReport, validate_findings_contract};
use crate::findings::types::Finding;
use crate::scan::types::ScanDiagnostic;
use std::time::Instant;

pub struct FindingContractValidationStage {
    pub elapsed_us: u64,
    pub report: FindingContractReport,
    pub diagnostics: Vec<ScanDiagnostic>,
}

pub fn validate_finding_contract_stage(findings: &[Finding]) -> FindingContractValidationStage {
    let start = Instant::now();
    let report = validate_findings_contract(findings);
    let elapsed_us = start.elapsed().as_micros() as u64;
    let diagnostics = finding_contract_diagnostics(&report);

    FindingContractValidationStage {
        elapsed_us,
        report,
        diagnostics,
    }
}
