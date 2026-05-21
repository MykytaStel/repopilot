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
