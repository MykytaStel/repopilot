use crate::audits::architecture::deep_nesting::DeepNestingAudit;
use crate::audits::architecture::large_file::LargeFileAudit;
use crate::audits::architecture::too_many_modules::TooManyModulesAudit;
use crate::audits::code_quality::code_markers::CodeMarkerAudit;
use crate::audits::code_quality::complexity::ComplexityAudit;
use crate::audits::code_quality::long_function::LongFunctionAudit;
use crate::audits::security::env_file_committed::EnvFileCommittedAudit;
use crate::audits::security::private_key_candidate::PrivateKeyCandidateAudit;
use crate::audits::security::secret_candidate::SecretCandidateAudit;
use crate::audits::testing::missing_test_folder::MissingTestFolderAudit;
use crate::audits::testing::source_without_test::SourceWithoutTestAudit;
use crate::audits::traits::{FileAudit, ProjectAudit};
use crate::findings::types::Finding;
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;

pub fn build_file_audits() -> Vec<Box<dyn FileAudit>> {
    vec![
        Box::new(LargeFileAudit),
        Box::new(CodeMarkerAudit),
        Box::new(SecretCandidateAudit),
        Box::new(PrivateKeyCandidateAudit),
        Box::new(ComplexityAudit),
        Box::new(LongFunctionAudit),
    ]
}

pub fn run_project_audits(scan_facts: &ScanFacts, config: &ScanConfig) -> Vec<Finding> {
    let project_audits: Vec<Box<dyn ProjectAudit>> = vec![
        Box::new(TooManyModulesAudit),
        Box::new(DeepNestingAudit),
        Box::new(MissingTestFolderAudit),
        Box::new(SourceWithoutTestAudit),
        Box::new(EnvFileCommittedAudit),
    ];

    project_audits
        .iter()
        .flat_map(|a| a.audit(scan_facts, config))
        .collect()
}

