#[test]
fn old_arch_flagged_when_no_app_json() {
    let dir = tempdir().unwrap();
    let findings = ReactNativeOldArchAudit.audit(&facts_for(dir.path()), &ScanConfig::default());
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
