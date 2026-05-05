use crate::audits::architecture::deep_nesting::DeepNestingAudit;
use crate::audits::architecture::large_file::LargeFileAudit;
use crate::audits::architecture::too_many_modules::TooManyModulesAudit;
use crate::audits::code_quality::code_markers::CodeMarkerAudit;
use crate::audits::security::env_file_committed::EnvFileCommittedAudit;
use crate::audits::security::private_key_candidate::PrivateKeyCandidateAudit;
use crate::audits::security::secret_candidate::SecretCandidateAudit;
use crate::audits::testing::missing_test_folder::MissingTestFolderAudit;
use crate::audits::testing::source_without_test::SourceWithoutTestAudit;
use crate::audits::traits::{FileAudit, ProjectAudit};
use crate::findings::types::Finding;
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;

pub fn run_audits(scan_facts: &ScanFacts, config: &ScanConfig) -> Vec<Finding> {
    let file_audits: Vec<Box<dyn FileAudit>> = vec![
        Box::new(LargeFileAudit),
        Box::new(CodeMarkerAudit),
        Box::new(SecretCandidateAudit),
        Box::new(PrivateKeyCandidateAudit),
    ];

    let project_audits: Vec<Box<dyn ProjectAudit>> = vec![
        Box::new(TooManyModulesAudit),
        Box::new(DeepNestingAudit),
        Box::new(MissingTestFolderAudit),
        Box::new(SourceWithoutTestAudit),
        Box::new(EnvFileCommittedAudit),
    ];

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
