use super::registration::{FrameworkAuditRegistration, RN_DEP_HEALTH_RULES, framework_metadata};
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
use crate::findings::types::Finding;
use crate::frameworks::DetectedFramework;
use crate::knowledge::decision::apply_project_decisions;
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;
use rayon::prelude::*;

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
        .par_iter()
        .map(|registration| registration.run(facts, config))
        .collect::<Vec<_>>()
        .into_iter()
        .flatten()
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
