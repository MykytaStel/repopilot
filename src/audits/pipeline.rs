use crate::audits::architecture::barrel_file_risk::BarrelFileRiskAudit;
use crate::audits::architecture::deep_nesting::DeepNestingAudit;
use crate::audits::architecture::deep_relative_imports::DeepRelativeImportsAudit;
use crate::audits::architecture::large_file::LargeFileAudit;
use crate::audits::architecture::too_many_modules::TooManyModulesAudit;
use crate::audits::code_quality::code_markers::CodeMarkerAudit;
use crate::audits::code_quality::complexity::ComplexityAudit;
use crate::audits::code_quality::language_risk::LanguageRiskAudit;
use crate::audits::code_quality::long_function::LongFunctionAudit;
use crate::audits::code_quality::rust_panic_risk::RustPanicRiskAudit;
use crate::audits::framework::django::{
    DjangoDebugTrueAudit, DjangoEmptyAllowedHostsAudit, DjangoRawSqlAudit,
};
use crate::audits::framework::js_common::{ConsoleLogAudit, VarDeclarationAudit};
use crate::audits::framework::react::{ReactClassComponentAudit, ReactPropTypesAudit};
use crate::audits::framework::react_native::{
    AsyncStorageFromCoreAudit, DirectStateMutationAudit, HermesDisabledAudit, HermesMismatchAudit,
    ReactNativeArchitectureMismatchAudit, ReactNativeCodegenMissingAudit, ReactNativeOldArchAudit,
    ReactNavigationV4Audit, RnDeprecatedApiAudit, RnFlatListMissingKeyAudit, RnInlineStyleAudit,
};
use crate::audits::framework::rn_dep_health::RnDepHealthAudit;
use crate::audits::metadata::{AuditKind, AuditMetadata};
use crate::audits::security::env_file_committed::EnvFileCommittedAudit;
use crate::audits::security::private_key_candidate::PrivateKeyCandidateAudit;
use crate::audits::security::secret_candidate::SecretCandidateAudit;
use crate::audits::testing::missing_test_folder::MissingTestFolderAudit;
use crate::audits::testing::source_without_test::SourceWithoutTestAudit;
use crate::audits::traits::{FileAudit, ProjectAudit};
use crate::findings::types::{Finding, FindingCategory};
use crate::frameworks::DetectedFramework;
use crate::knowledge::decision::apply_project_decisions;
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;

const CODE_MARKER_RULES: &[&str] = &["code-marker.todo", "code-marker.fixme", "code-marker.hack"];
const LANGUAGE_RISK_RULES: &[&str] = &[
    "language.go.panic-exit-risk",
    "language.python.exception-risk",
    "language.javascript.runtime-exit-risk",
    "language.managed.fatal-exception-risk",
];
const RN_DEP_HEALTH_RULES: &[&str] = &[
    "framework.rn-async-storage-legacy",
    "framework.rn-navigation-compat",
    "framework.rn-reanimated-compat",
    "framework.rn-gesture-handler-old",
    "framework.rn-new-arch-incompatible-dep",
];

pub struct FileAuditRegistration {
    pub metadata: AuditMetadata,
    audit: Box<dyn FileAudit>,
}

impl FileAuditRegistration {
    fn new(metadata: AuditMetadata, audit: Box<dyn FileAudit>) -> Self {
        Self { metadata, audit }
    }

    fn into_audit(self) -> Box<dyn FileAudit> {
        self.audit
    }
}

pub struct ProjectAuditRegistration {
    pub metadata: AuditMetadata,
    audit: Box<dyn ProjectAudit>,
}

impl ProjectAuditRegistration {
    fn new(metadata: AuditMetadata, audit: Box<dyn ProjectAudit>) -> Self {
        Self { metadata, audit }
    }
}

pub struct FrameworkAuditRegistration {
    pub metadata: AuditMetadata,
    audit: Box<dyn ProjectAudit>,
}

impl FrameworkAuditRegistration {
    fn new(metadata: AuditMetadata, audit: Box<dyn ProjectAudit>) -> Self {
        Self { metadata, audit }
    }
}

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
                "audit.project.architecture.deep-nesting",
                FindingCategory::Architecture,
                &["architecture.deep-nesting"],
            ),
            Box::new(DeepNestingAudit),
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
        .iter()
        .flat_map(|registration| registration.audit.audit(scan_facts, config))
        .collect();

    apply_project_decisions(scan_facts, findings)
}

pub fn registered_framework_audits(facts: &ScanFacts) -> Vec<FrameworkAuditRegistration> {
    let scope = detect_framework_scope(facts);
    let mut audits = Vec::new();

    if scope.has_rn {
        audits.extend([
            FrameworkAuditRegistration::new(
                framework_metadata(
                    "audit.framework.react-native.old-architecture",
                    &["framework.react-native.old-architecture"],
                ),
                Box::new(ReactNativeOldArchAudit),
            ),
            FrameworkAuditRegistration::new(
                framework_metadata(
                    "audit.framework.react-native.architecture-mismatch",
                    &["framework.react-native.architecture-mismatch"],
                ),
                Box::new(ReactNativeArchitectureMismatchAudit),
            ),
            FrameworkAuditRegistration::new(
                framework_metadata(
                    "audit.framework.react-native.async-storage-from-core",
                    &["framework.react-native.async-storage-from-core"],
                ),
                Box::new(AsyncStorageFromCoreAudit),
            ),
            FrameworkAuditRegistration::new(
                framework_metadata(
                    "audit.framework.react-native.hermes-disabled",
                    &["framework.react-native.hermes-disabled"],
                ),
                Box::new(HermesDisabledAudit),
            ),
            FrameworkAuditRegistration::new(
                framework_metadata(
                    "audit.framework.react-native.hermes-mismatch",
                    &["framework.react-native.hermes-mismatch"],
                ),
                Box::new(HermesMismatchAudit),
            ),
            FrameworkAuditRegistration::new(
                framework_metadata(
                    "audit.framework.react-native.codegen-missing",
                    &["framework.react-native.codegen-missing"],
                ),
                Box::new(ReactNativeCodegenMissingAudit),
            ),
            FrameworkAuditRegistration::new(
                framework_metadata(
                    "audit.framework.react-native.old-react-navigation",
                    &["framework.react-native.old-react-navigation"],
                ),
                Box::new(ReactNavigationV4Audit),
            ),
            FrameworkAuditRegistration::new(
                framework_metadata(
                    "audit.framework.react-native.direct-state-mutation",
                    &["framework.react-native.direct-state-mutation"],
                ),
                Box::new(DirectStateMutationAudit),
            ),
            FrameworkAuditRegistration::new(
                framework_metadata(
                    "audit.framework.react-native.dependency-health",
                    RN_DEP_HEALTH_RULES,
                ),
                Box::new(RnDepHealthAudit),
            ),
            FrameworkAuditRegistration::new(
                framework_metadata(
                    "audit.framework.react-native.inline-style",
                    &["framework.react-native.inline-style"],
                ),
                Box::new(RnInlineStyleAudit),
            ),
            FrameworkAuditRegistration::new(
                framework_metadata(
                    "audit.framework.react-native.deprecated-api",
                    &["framework.react-native.deprecated-api"],
                ),
                Box::new(RnDeprecatedApiAudit),
            ),
            FrameworkAuditRegistration::new(
                framework_metadata(
                    "audit.framework.react-native.flatlist-missing-key",
                    &["framework.react-native.flatlist-missing-key"],
                ),
                Box::new(RnFlatListMissingKeyAudit),
            ),
        ]);
    }

    if scope.has_react_only {
        audits.extend([
            FrameworkAuditRegistration::new(
                framework_metadata(
                    "audit.framework.react.class-component",
                    &["framework.react.class-component"],
                ),
                Box::new(ReactClassComponentAudit),
            ),
            FrameworkAuditRegistration::new(
                framework_metadata(
                    "audit.framework.react.prop-types",
                    &["framework.react.prop-types"],
                ),
                Box::new(ReactPropTypesAudit),
            ),
        ]);
    }

    if scope.has_django {
        audits.extend([
            FrameworkAuditRegistration::new(
                framework_metadata(
                    "audit.framework.django.debug-true",
                    &["framework.django.debug-true"],
                ),
                Box::new(DjangoDebugTrueAudit),
            ),
            FrameworkAuditRegistration::new(
                framework_metadata(
                    "audit.framework.django.missing-allowed-hosts",
                    &["framework.django.missing-allowed-hosts"],
                ),
                Box::new(DjangoEmptyAllowedHostsAudit),
            ),
            FrameworkAuditRegistration::new(
                framework_metadata(
                    "audit.framework.django.raw-sql-query",
                    &["framework.django.raw-sql-query"],
                ),
                Box::new(DjangoRawSqlAudit),
            ),
        ]);
    }

    if scope.has_js {
        audits.extend([
            FrameworkAuditRegistration::new(
                framework_metadata(
                    "audit.framework.js.var-declaration",
                    &["framework.js.var-declaration"],
                ),
                Box::new(VarDeclarationAudit),
            ),
            FrameworkAuditRegistration::new(
                framework_metadata(
                    "audit.framework.js.console-log",
                    &["framework.js.console-log"],
                ),
                Box::new(ConsoleLogAudit),
            ),
        ]);
    }

    audits
}

pub fn run_framework_audits(facts: &ScanFacts, config: &ScanConfig) -> Vec<Finding> {
    let findings = registered_framework_audits(facts)
        .iter()
        .flat_map(|registration| registration.audit.audit(facts, config))
        .collect();

    apply_project_decisions(facts, findings)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FrameworkScope {
    has_rn: bool,
    has_react_only: bool,
    has_js: bool,
    has_django: bool,
}

fn detect_framework_scope(facts: &ScanFacts) -> FrameworkScope {
    let has_rn = facts
        .detected_frameworks
        .iter()
        .any(|f| matches!(f, DetectedFramework::ReactNative { .. }))
        || facts.framework_projects.iter().any(|project| {
            project
                .frameworks
                .iter()
                .any(|f| matches!(f, DetectedFramework::ReactNative { .. }))
        });
    let has_react = facts
        .detected_frameworks
        .iter()
        .any(|f| matches!(f, DetectedFramework::React { .. }))
        || facts.framework_projects.iter().any(|project| {
            project
                .frameworks
                .iter()
                .any(|f| matches!(f, DetectedFramework::React { .. }))
        });
    // React web audits run only when React is present but React Native is not —
    // RN projects always declare `react` as a peer dep, so without this guard
    // web-focused checks (class components, prop-types) would run on every RN project.
    let has_react_only = has_react && !has_rn;

    let is_js_framework = |f: &DetectedFramework| {
        matches!(
            f,
            DetectedFramework::ReactNative { .. }
                | DetectedFramework::Expo { .. }
                | DetectedFramework::React { .. }
                | DetectedFramework::NextJs { .. }
                | DetectedFramework::Vue { .. }
                | DetectedFramework::Angular { .. }
                | DetectedFramework::Svelte { .. }
                | DetectedFramework::NestJs { .. }
                | DetectedFramework::Express { .. }
        )
    };
    let has_js = facts.detected_frameworks.iter().any(is_js_framework)
        || facts
            .framework_projects
            .iter()
            .any(|project| project.frameworks.iter().any(is_js_framework));

    let has_django = facts
        .detected_frameworks
        .iter()
        .any(|f| matches!(f, DetectedFramework::Django { .. }));

    FrameworkScope {
        has_rn,
        has_react_only,
        has_js,
        has_django,
    }
}

fn file_metadata(
    audit_id: &'static str,
    category: FindingCategory,
    rule_ids: &'static [&'static str],
) -> AuditMetadata {
    AuditMetadata::new(audit_id, AuditKind::File, category, rule_ids)
}

fn project_metadata(
    audit_id: &'static str,
    category: FindingCategory,
    rule_ids: &'static [&'static str],
) -> AuditMetadata {
    AuditMetadata::new(audit_id, AuditKind::Project, category, rule_ids)
}

fn framework_metadata(audit_id: &'static str, rule_ids: &'static [&'static str]) -> AuditMetadata {
    AuditMetadata::new(
        audit_id,
        AuditKind::Framework,
        FindingCategory::Framework,
        rule_ids,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::lookup_rule_metadata;
    use crate::scan::facts::ScanFacts;
    use std::collections::HashSet;

    #[test]
    fn registered_file_audits_have_rule_metadata() {
        let config = ScanConfig {
            detect_secret_like_names: true,
            ..ScanConfig::default()
        };

        assert_registrations_have_rule_metadata(
            registered_file_audits(&config)
                .iter()
                .map(|registration| registration.metadata.clone()),
            AuditKind::File,
        );
    }

    #[test]
    fn registered_project_audits_have_rule_metadata() {
        let config = ScanConfig {
            detect_missing_tests: true,
            ..ScanConfig::default()
        };

        assert_registrations_have_rule_metadata(
            registered_project_audits(&config)
                .iter()
                .map(|registration| registration.metadata.clone()),
            AuditKind::Project,
        );
    }

    #[test]
    fn registered_framework_audits_have_rule_metadata() {
        let facts = ScanFacts {
            detected_frameworks: vec![
                DetectedFramework::ReactNative { version: None },
                DetectedFramework::React { version: None },
            ],
            ..ScanFacts::default()
        };

        assert_registrations_have_rule_metadata(
            registered_framework_audits(&facts)
                .iter()
                .map(|registration| registration.metadata.clone()),
            AuditKind::Framework,
        );
    }

    #[test]
    fn registered_audit_ids_are_unique_within_each_scope() {
        let config = ScanConfig {
            detect_missing_tests: true,
            detect_secret_like_names: true,
            ..ScanConfig::default()
        };
        let framework_facts = ScanFacts {
            detected_frameworks: vec![
                DetectedFramework::ReactNative { version: None },
                DetectedFramework::React { version: None },
            ],
            ..ScanFacts::default()
        };

        assert_unique_audit_ids(
            registered_file_audits(&config)
                .iter()
                .map(|registration| registration.metadata.clone()),
        );
        assert_unique_audit_ids(
            registered_project_audits(&config)
                .iter()
                .map(|registration| registration.metadata.clone()),
        );
        assert_unique_audit_ids(
            registered_framework_audits(&framework_facts)
                .iter()
                .map(|registration| registration.metadata.clone()),
        );
    }

    fn assert_registrations_have_rule_metadata(
        registrations: impl Iterator<Item = AuditMetadata>,
        expected_kind: AuditKind,
    ) {
        for metadata in registrations {
            assert_eq!(
                metadata.kind, expected_kind,
                "audit {} has unexpected kind",
                metadata.audit_id
            );
            assert!(
                !metadata.rule_ids.is_empty(),
                "audit {} should declare at least one rule_id",
                metadata.audit_id
            );

            for rule_id in metadata.rule_ids {
                let rule = lookup_rule_metadata(rule_id).unwrap_or_else(|| {
                    panic!(
                        "audit {} references missing rule metadata: {}",
                        metadata.audit_id, rule_id
                    )
                });
                assert_eq!(
                    rule.category, metadata.category,
                    "audit {} category does not match rule {}",
                    metadata.audit_id, rule_id
                );
            }
        }
    }

    fn assert_unique_audit_ids(registrations: impl Iterator<Item = AuditMetadata>) {
        let mut seen = HashSet::new();

        for metadata in registrations {
            assert!(
                seen.insert(metadata.audit_id),
                "duplicate audit_id: {}",
                metadata.audit_id
            );
        }
    }
}
