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

pub fn build_file_audits(config: &ScanConfig) -> Vec<Box<dyn FileAudit>> {
    let mut audits: Vec<Box<dyn FileAudit>> = vec![
        Box::new(LargeFileAudit),
        Box::new(CodeMarkerAudit),
        Box::new(PrivateKeyCandidateAudit),
        Box::new(ComplexityAudit),
        Box::new(LongFunctionAudit),
    ];

    if config.detect_secret_like_names {
        audits.insert(2, Box::new(SecretCandidateAudit));
    }

    audits
}

pub fn run_project_audits(scan_facts: &ScanFacts, config: &ScanConfig) -> Vec<Finding> {
    let mut project_audits: Vec<Box<dyn ProjectAudit>> = vec![
        Box::new(TooManyModulesAudit),
        Box::new(DeepNestingAudit),
        Box::new(EnvFileCommittedAudit),
    ];

    if config.detect_missing_tests {
        project_audits.insert(2, Box::new(SourceWithoutTestAudit));
        project_audits.insert(2, Box::new(MissingTestFolderAudit));
    }

    project_audits
        .iter()
        .flat_map(|a| a.audit(scan_facts, config))
        .collect()
}
