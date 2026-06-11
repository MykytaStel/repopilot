use super::registration::{
    CODE_MARKER_RULES, FileAuditRegistration, LANGUAGE_RISK_RULES, file_metadata,
};
use crate::audits::architecture::large_file::LargeFileAudit;
use crate::audits::code_quality::code_markers::CodeMarkerAudit;
use crate::audits::code_quality::complex_function::ComplexFunctionAudit;
use crate::audits::code_quality::complexity::ComplexityAudit;
use crate::audits::code_quality::deep_control_flow::DeepControlFlowAudit;
use crate::audits::code_quality::language_risk::LanguageRiskAudit;
use crate::audits::code_quality::long_function::LongFunctionAudit;
use crate::audits::code_quality::rust_panic_risk::RustPanicRiskAudit;
use crate::audits::security::private_key_candidate::PrivateKeyCandidateAudit;
use crate::audits::security::secret_candidate::SecretCandidateAudit;
use crate::audits::traits::FileAudit;
use crate::findings::types::FindingCategory;
use crate::scan::config::ScanConfig;

pub fn registered_file_audits(config: &ScanConfig) -> Vec<FileAuditRegistration> {
    let mut audits = vec![
        FileAuditRegistration::new(
            file_metadata(
                "audit.file.architecture.large-file",
                FindingCategory::Architecture,
                &["architecture.large-file"],
            ),
            Box::new(LargeFileAudit),
        ),
        FileAuditRegistration::new(
            file_metadata(
                "audit.file.code-quality.code-markers",
                FindingCategory::CodeQuality,
                CODE_MARKER_RULES,
            ),
            Box::new(CodeMarkerAudit),
        ),
        FileAuditRegistration::new(
            file_metadata(
                "audit.file.security.private-key-candidate",
                FindingCategory::Security,
                &["security.private-key-candidate"],
            ),
            Box::new(PrivateKeyCandidateAudit),
        ),
        FileAuditRegistration::new(
            file_metadata(
                "audit.file.code-quality.complexity",
                FindingCategory::CodeQuality,
                &["code-quality.complex-file"],
            ),
            Box::new(ComplexityAudit),
        ),
        FileAuditRegistration::new(
            file_metadata(
                "audit.file.code-quality.deep-control-flow",
                FindingCategory::CodeQuality,
                &["code-quality.deep-control-flow"],
            ),
            Box::new(DeepControlFlowAudit),
        ),
        FileAuditRegistration::new(
            file_metadata(
                "audit.file.code-quality.complex-function",
                FindingCategory::CodeQuality,
                &["code-quality.complex-function"],
            ),
            Box::new(ComplexFunctionAudit),
        ),
        FileAuditRegistration::new(
            file_metadata(
                "audit.file.code-quality.long-function",
                FindingCategory::CodeQuality,
                &["code-quality.long-function"],
            ),
            Box::new(LongFunctionAudit),
        ),
        FileAuditRegistration::new(
            file_metadata(
                "audit.file.language.rust-panic-risk",
                FindingCategory::CodeQuality,
                &["language.rust.panic-risk"],
            ),
            Box::new(RustPanicRiskAudit),
        ),
        FileAuditRegistration::new(
            file_metadata(
                "audit.file.language.runtime-risk",
                FindingCategory::CodeQuality,
                LANGUAGE_RISK_RULES,
            ),
            Box::new(LanguageRiskAudit),
        ),
    ];

    if config.detect_secret_like_names {
        audits.insert(
            2,
            FileAuditRegistration::new(
                file_metadata(
                    "audit.file.security.secret-candidate",
                    FindingCategory::Security,
                    &["security.secret-candidate"],
                ),
                Box::new(SecretCandidateAudit),
            ),
        );
    }

    audits
}

pub fn build_file_audits(config: &ScanConfig) -> Vec<Box<dyn FileAudit>> {
    registered_file_audits(config)
        .into_iter()
        .map(FileAuditRegistration::into_audit)
        .collect()
}
