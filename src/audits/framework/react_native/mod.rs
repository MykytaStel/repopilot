mod architecture;
mod async_storage;
mod hermes;
mod navigation;
mod styling;

pub use architecture::{
    HermesMismatchAudit, ReactNativeArchitectureMismatchAudit, ReactNativeOldArchAudit,
};
pub use async_storage::AsyncStorageFromCoreAudit;
pub use hermes::{HermesDisabledAudit, ReactNativeCodegenMissingAudit};
pub use navigation::{DirectStateMutationAudit, ReactNavigationV4Audit};
pub use styling::{RnDeprecatedApiAudit, RnFlatListMissingKeyAudit, RnInlineStyleAudit};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audits::traits::ProjectAudit;
    use crate::findings::types::Severity;
    use crate::frameworks::ReactNativeArchitectureProfile;
    use crate::scan::config::ScanConfig;
    use crate::scan::facts::{FileFacts, ScanFacts};
    use std::io::Write;
    use tempfile::tempdir;

    fn facts_for(root: &std::path::Path) -> ScanFacts {
        ScanFacts {
            root_path: root.to_path_buf(),
            react_native: Some(ReactNativeArchitectureProfile {
                detected: true,
                ..ReactNativeArchitectureProfile::default()
            }),
            ..ScanFacts::default()
        }
    }

    fn jsx_file(dir: &tempfile::TempDir, name: &str, content: &str) -> FileFacts {
        let path = dir.path().join(name);
        write!(std::fs::File::create(&path).unwrap(), "{content}").unwrap();
        FileFacts {
            path,
            language: Some("TypeScript React".to_string()),
            lines_of_code: content.lines().count(),
            branch_count: 0,
            imports: vec![],
            content: String::new(),
        }
    }

    // ── Old architecture ──────────────────────────────────────────────────────

    #[test]
    fn old_arch_flagged_when_no_app_json() {
        let dir = tempdir().unwrap();
        let findings =
            ReactNativeOldArchAudit.audit(&facts_for(dir.path()), &ScanConfig::default());
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].rule_id,
            "framework.react-native.old-architecture"
        );
    }

    #[test]
    fn old_arch_not_flagged_when_enabled() {
        let dir = tempdir().unwrap();
        write!(
            std::fs::File::create(dir.path().join("app.json")).unwrap(),
            r#"{{"expo": {{"newArchEnabled": true}}}}"#
        )
        .unwrap();
        let mut facts = facts_for(dir.path());
        facts.react_native = Some(ReactNativeArchitectureProfile {
            detected: true,
            has_expo_config: true,
            expo_new_arch_enabled: Some(true),
            ..ReactNativeArchitectureProfile::default()
        });
        let findings = ReactNativeOldArchAudit.audit(&facts, &ScanConfig::default());
        assert!(findings.is_empty());
    }

    #[test]
    fn old_arch_not_flagged_when_enabled_in_rn_config() {
        let dir = tempdir().unwrap();
        writeln!(
            std::fs::File::create(dir.path().join("react-native.config.js")).unwrap(),
            "module.exports = {{ newArchEnabled: true }};"
        )
        .unwrap();
        let mut facts = facts_for(dir.path());
        facts.react_native = Some(ReactNativeArchitectureProfile {
            detected: true,
            expo_new_arch_enabled: Some(true),
            ..ReactNativeArchitectureProfile::default()
        });
        let findings = ReactNativeOldArchAudit.audit(&facts, &ScanConfig::default());
        assert!(findings.is_empty());
    }

    #[test]
    fn architecture_mismatch_is_flagged() {
        let dir = tempdir().unwrap();
        let mut facts = facts_for(dir.path());
        facts.react_native = Some(ReactNativeArchitectureProfile {
            detected: true,
            android_new_arch_enabled: Some(false),
            ios_new_arch_enabled: Some(true),
            architecture_mismatch: true,
            ..ReactNativeArchitectureProfile::default()
        });

        let findings = ReactNativeArchitectureMismatchAudit.audit(&facts, &ScanConfig::default());

        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].rule_id,
            "framework.react-native.architecture-mismatch"
        );
        assert_eq!(findings[0].severity, Severity::High);
    }

    #[test]
    fn hermes_mismatch_is_flagged() {
        let dir = tempdir().unwrap();
        let mut facts = facts_for(dir.path());
        facts.react_native = Some(ReactNativeArchitectureProfile {
            detected: true,
            android_hermes_enabled: Some(true),
            ios_hermes_enabled: Some(false),
            hermes_mismatch: true,
            ..ReactNativeArchitectureProfile::default()
        });

        let findings = HermesMismatchAudit.audit(&facts, &ScanConfig::default());

        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].rule_id,
            "framework.react-native.hermes-mismatch"
        );
        assert_eq!(findings[0].severity, Severity::Medium);
    }

    #[test]
    fn codegen_missing_is_flagged_when_turbo_module_signal_exists() {
        let dir = tempdir().unwrap();
        let mut facts = facts_for(dir.path());
        facts.react_native = Some(ReactNativeArchitectureProfile {
            detected: true,
            has_codegen_config: false,
            ..ReactNativeArchitectureProfile::default()
        });
        facts.files.push(jsx_file(
            &dir,
            "NativeLocalStorage.ts",
            "import type { TurboModule } from 'react-native';\nexport interface Spec extends TurboModule {}\n",
        ));

        let findings = ReactNativeCodegenMissingAudit.audit(&facts, &ScanConfig::default());

        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].rule_id,
            "framework.react-native.codegen-missing"
        );
    }

    #[test]
    fn codegen_missing_is_skipped_when_config_exists() {
        let dir = tempdir().unwrap();
        let mut facts = facts_for(dir.path());
        facts.react_native = Some(ReactNativeArchitectureProfile {
            detected: true,
            has_codegen_config: true,
            ..ReactNativeArchitectureProfile::default()
        });
        facts.files.push(jsx_file(
            &dir,
            "NativeLocalStorage.ts",
            "import type { TurboModule } from 'react-native';\nexport interface Spec extends TurboModule {}\n",
        ));

        let findings = ReactNativeCodegenMissingAudit.audit(&facts, &ScanConfig::default());

        assert!(findings.is_empty());
    }

    // ── AsyncStorage ──────────────────────────────────────────────────────────

    #[test]
    fn async_storage_single_line_detected() {
        let dir = tempdir().unwrap();
        let mut facts = facts_for(dir.path());
        facts.files.push(jsx_file(
            &dir,
            "Screen.tsx",
            "import { View, AsyncStorage, Text } from 'react-native';\n",
        ));
        let findings = AsyncStorageFromCoreAudit.audit(&facts, &ScanConfig::default());
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].rule_id,
            "framework.react-native.async-storage-from-core"
        );
        assert_eq!(findings[0].severity, Severity::High);
    }

    #[test]
    fn async_storage_multi_line_detected() {
        let dir = tempdir().unwrap();
        let mut facts = facts_for(dir.path());
        facts.files.push(jsx_file(
            &dir,
            "Screen.tsx",
            "import {\n  View,\n  AsyncStorage,\n  Text,\n} from 'react-native';\n",
        ));
        let findings = AsyncStorageFromCoreAudit.audit(&facts, &ScanConfig::default());
        assert_eq!(findings.len(), 1, "multi-line import must be detected");
        assert_eq!(findings[0].evidence[0].line_start, 1);
    }

    #[test]
    fn async_storage_from_own_package_not_flagged() {
        let dir = tempdir().unwrap();
        let mut facts = facts_for(dir.path());
        facts.files.push(jsx_file(
            &dir,
            "Screen.tsx",
            "import AsyncStorage from '@react-native-async-storage/async-storage';\n",
        ));
        let findings = AsyncStorageFromCoreAudit.audit(&facts, &ScanConfig::default());
        assert!(findings.is_empty());
    }

    // ── React Navigation v4 ───────────────────────────────────────────────────

    #[test]
    fn old_react_navigation_detected() {
        let dir = tempdir().unwrap();
        let mut facts = facts_for(dir.path());
        facts.files.push(jsx_file(
            &dir,
            "Navigator.tsx",
            "import { createStackNavigator } from 'react-navigation';\n",
        ));
        let findings = ReactNavigationV4Audit.audit(&facts, &ScanConfig::default());
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].rule_id,
            "framework.react-native.old-react-navigation"
        );
        assert_eq!(findings[0].severity, Severity::Medium);
    }

    #[test]
    fn modern_react_navigation_not_flagged() {
        let dir = tempdir().unwrap();
        let mut facts = facts_for(dir.path());
        facts.files.push(jsx_file(
            &dir,
            "Navigator.tsx",
            "import { NavigationContainer } from '@react-navigation/native';\n",
        ));
        let findings = ReactNavigationV4Audit.audit(&facts, &ScanConfig::default());
        assert!(findings.is_empty());
    }

    // ── Direct state mutation ─────────────────────────────────────────────────

    #[test]
    fn direct_state_mutation_detected() {
        let dir = tempdir().unwrap();
        let mut facts = facts_for(dir.path());
        facts.files.push(jsx_file(
            &dir,
            "Comp.tsx",
            "class MyComp extends React.Component {\n  handleClick() {\n    this.state.count = 5;\n  }\n}\n",
        ));
        let findings = DirectStateMutationAudit.audit(&facts, &ScanConfig::default());
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].rule_id,
            "framework.react-native.direct-state-mutation"
        );
        assert_eq!(findings[0].severity, Severity::High);
    }

    #[test]
    fn state_equality_check_not_flagged() {
        let dir = tempdir().unwrap();
        let mut facts = facts_for(dir.path());
        facts.files.push(jsx_file(
            &dir,
            "Comp.tsx",
            "if (this.state.count === 5) { doSomething(); }\n",
        ));
        let findings = DirectStateMutationAudit.audit(&facts, &ScanConfig::default());
        assert!(findings.is_empty());
    }

    // ── Hermes disabled via gradle.properties ─────────────────────────────────

    #[test]
    fn hermes_disabled_in_gradle_properties_is_flagged() {
        let dir = tempdir().unwrap();
        let android = dir.path().join("android");
        std::fs::create_dir(&android).unwrap();
        write!(
            std::fs::File::create(android.join("gradle.properties")).unwrap(),
            "hermesEnabled=false\nnewArchEnabled=true\n"
        )
        .unwrap();

        let findings = HermesDisabledAudit.audit(&facts_for(dir.path()), &ScanConfig::default());
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].rule_id,
            "framework.react-native.hermes-disabled"
        );
    }

    #[test]
    fn hermes_enabled_in_gradle_properties_is_not_flagged() {
        let dir = tempdir().unwrap();
        let android = dir.path().join("android");
        std::fs::create_dir(&android).unwrap();
        writeln!(
            std::fs::File::create(android.join("gradle.properties")).unwrap(),
            "hermesEnabled=true"
        )
        .unwrap();

        let findings = HermesDisabledAudit.audit(&facts_for(dir.path()), &ScanConfig::default());
        assert!(findings.is_empty());
    }

    #[test]
    fn hermes_disabled_gradle_properties_with_inline_comment_is_flagged() {
        let dir = tempdir().unwrap();
        let android = dir.path().join("android");
        std::fs::create_dir(&android).unwrap();
        writeln!(
            std::fs::File::create(android.join("gradle.properties")).unwrap(),
            "hermesEnabled=false   # JSC is faster for our use case"
        )
        .unwrap();

        let findings = HermesDisabledAudit.audit(&facts_for(dir.path()), &ScanConfig::default());
        assert_eq!(findings.len(), 1);
    }
}
