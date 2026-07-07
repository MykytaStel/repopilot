use super::registration::{ProjectAuditRegistration, project_metadata};
use crate::audits::architecture::barrel_file_risk::BarrelFileRiskAudit;
use crate::audits::architecture::deep_directory_nesting::DeepDirectoryNestingAudit;
use crate::audits::architecture::deep_relative_imports::DeepRelativeImportsAudit;
use crate::audits::architecture::too_many_modules::TooManyModulesAudit;
use crate::audits::security::env_file_committed::EnvFileCommittedAudit;
use crate::audits::testing::missing_test_folder::MissingTestFolderAudit;
use crate::audits::testing::source_without_test::SourceWithoutTestAudit;
use crate::findings::types::{Finding, FindingCategory};
use crate::knowledge::decision::apply_project_decisions;
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;
use rayon::prelude::*;

pub fn registered_project_audits(config: &ScanConfig) -> Vec<ProjectAuditRegistration> {
    let mut audits = vec![
        ProjectAuditRegistration::new(
            project_metadata(
                "audit.project.architecture.too-many-modules",
                FindingCategory::Architecture,
                &["architecture.too-many-modules"],
            ),
            Box::new(TooManyModulesAudit),
        ),
        ProjectAuditRegistration::new(
            project_metadata(
                "audit.project.architecture.deep-directory-nesting",
                FindingCategory::Architecture,
                &["architecture.deep-directory-nesting"],
            ),
            Box::new(DeepDirectoryNestingAudit),
        ),
        ProjectAuditRegistration::new(
            project_metadata(
                "audit.project.architecture.deep-relative-imports",
                FindingCategory::Architecture,
                &["architecture.deep-relative-imports"],
            ),
            Box::new(DeepRelativeImportsAudit),
        ),
        ProjectAuditRegistration::new(
            project_metadata(
                "audit.project.architecture.barrel-file-risk",
                FindingCategory::Architecture,
                &["architecture.barrel-file-risk"],
            ),
            Box::new(BarrelFileRiskAudit),
        ),
        ProjectAuditRegistration::new(
            project_metadata(
                "audit.project.security.env-file-committed",
                FindingCategory::Security,
                &["security.env-file-committed"],
            ),
            Box::new(EnvFileCommittedAudit),
        ),
    ];

    if config.detect_missing_tests {
        audits.insert(
            2,
            ProjectAuditRegistration::new(
                project_metadata(
                    "audit.project.testing.source-without-test",
                    FindingCategory::Testing,
                    &["testing.source-without-test"],
                ),
                Box::new(SourceWithoutTestAudit),
            ),
        );
        audits.insert(
            2,
            ProjectAuditRegistration::new(
                project_metadata(
                    "audit.project.testing.missing-test-folder",
                    FindingCategory::Testing,
                    &["testing.missing-test-folder"],
                ),
                Box::new(MissingTestFolderAudit),
            ),
        );
    }

    audits
}

pub fn run_project_audits(scan_facts: &ScanFacts, config: &ScanConfig) -> Vec<Finding> {
    let findings = registered_project_audits(config)
        .par_iter()
        .map(|registration| registration.run(scan_facts, config))
        .collect::<Vec<_>>()
        .into_iter()
        .flatten()
        .collect();

    apply_project_decisions(scan_facts, findings)
}
