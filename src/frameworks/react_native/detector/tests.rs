use super::*;
use crate::frameworks::react_native::profile::ReactNativeProjectKind;
use std::fs;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn detects_react_native_project() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    let mut f = fs::File::create(root.join("package.json")).unwrap();
    write!(
        f,
        r#"{{"dependencies": {{"react-native": "^0.73.0", "react": "18.2.0"}}}}"#
    )
    .unwrap();
    fs::create_dir(root.join("ios")).unwrap();
    fs::create_dir(root.join("android")).unwrap();

    let profile = detect_react_native_architecture(root);

    assert!(profile.detected);
    assert_eq!(profile.react_native_version, Some("0.73.0".to_string()));
    assert!(profile.has_ios);
    assert!(profile.has_android);
    assert_eq!(profile.project_kind, ReactNativeProjectKind::Bare);
}

#[test]
fn detects_android_new_arch_enabled_and_hermes() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    let mut f = fs::File::create(root.join("package.json")).unwrap();
    write!(f, r#"{{"dependencies": {{"react-native": "0.73.0"}}}}"#).unwrap();

    fs::create_dir(root.join("android")).unwrap();
    let mut gp = fs::File::create(root.join("android/gradle.properties")).unwrap();
    writeln!(gp, "newArchEnabled=true").unwrap();
    writeln!(gp, "hermesEnabled=true").unwrap();

    let profile = detect_react_native_architecture(root);

    assert_eq!(profile.android_new_arch_enabled, Some(true));
    assert_eq!(profile.android_hermes_enabled, Some(true));
    assert_eq!(profile.hermes_enabled, Some(true));
    assert!(profile.android_gradle_properties_found);
}

#[test]
fn detects_android_new_arch_disabled_with_spaces() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    let mut f = fs::File::create(root.join("package.json")).unwrap();
    write!(f, r#"{{"dependencies": {{"react-native": "0.73.0"}}}}"#).unwrap();

    fs::create_dir(root.join("android")).unwrap();
    let mut gp = fs::File::create(root.join("android/gradle.properties")).unwrap();
    writeln!(gp, "newArchEnabled = false").unwrap();

    let profile = detect_react_native_architecture(root);

    assert_eq!(profile.android_new_arch_enabled, Some(false));
}

#[test]
fn detects_codegen_config() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    let mut f = fs::File::create(root.join("package.json")).unwrap();
    write!(
        f,
        r#"{{
                "dependencies": {{"react-native": "0.73.0"}},
                "codegenConfig": {{"name": "ExampleSpec"}}
            }}"#
    )
    .unwrap();

    let profile = detect_react_native_architecture(root);

    assert!(profile.has_codegen_config);
}

#[test]
fn detects_ios_podfile_signals() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    let mut f = fs::File::create(root.join("package.json")).unwrap();
    write!(f, r#"{{"dependencies": {{"react-native": "0.73.0"}}}}"#).unwrap();

    fs::create_dir(root.join("ios")).unwrap();
    let mut pf = fs::File::create(root.join("ios/Podfile")).unwrap();
    writeln!(pf, "ENV['RCT_NEW_ARCH_ENABLED'] = '1'").unwrap();
    writeln!(pf, "use_react_native!(").unwrap();
    writeln!(pf, "  :hermes_enabled => true").unwrap();
    writeln!(pf, ")").unwrap();

    let profile = detect_react_native_architecture(root);

    assert_eq!(profile.ios_new_arch_enabled, Some(true));
    assert_eq!(profile.ios_hermes_enabled, Some(true));
    assert_eq!(profile.hermes_enabled, Some(true));
    assert!(profile.ios_podfile_found);
}

#[test]
fn detects_expo_config_and_project_kind() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    let mut f = fs::File::create(root.join("package.json")).unwrap();
    write!(
        f,
        r#"{{"dependencies": {{"react-native": "0.76.0", "expo": "53.0.0"}}}}"#
    )
    .unwrap();
    let mut app = fs::File::create(root.join("app.json")).unwrap();
    write!(app, r#"{{"expo": {{"newArchEnabled": true}}}}"#).unwrap();

    let profile = detect_react_native_architecture(root);

    assert_eq!(profile.project_kind, ReactNativeProjectKind::ExpoManaged);
    assert!(profile.has_expo_config);
    assert_eq!(profile.expo_new_arch_enabled, Some(true));
}

#[test]
fn detects_podfile_properties_json() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    let mut f = fs::File::create(root.join("package.json")).unwrap();
    write!(f, r#"{{"dependencies": {{"react-native": "0.76.0"}}}}"#).unwrap();
    fs::create_dir(root.join("ios")).unwrap();
    let mut props = fs::File::create(root.join("ios/Podfile.properties.json")).unwrap();
    write!(props, r#"{{"newArchEnabled": "true"}}"#).unwrap();

    let profile = detect_react_native_architecture(root);

    assert!(profile.ios_podfile_properties_found);
    assert_eq!(profile.ios_new_arch_enabled, Some(true));
}

#[test]
fn malformed_package_json_does_not_panic() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    let mut f = fs::File::create(root.join("package.json")).unwrap();
    write!(f, "{{ broken json").unwrap();

    let profile = detect_react_native_architecture(root);

    assert!(!profile.detected);
    assert_eq!(profile.react_native_version, None);
}

#[test]
fn non_rn_project_returns_not_detected() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    let mut f = fs::File::create(root.join("package.json")).unwrap();
    write!(
        f,
        r#"{{"dependencies": {{"react": "18.0.0", "next": "14.0.0"}}}}"#
    )
    .unwrap();

    let profile = detect_react_native_architecture(root);

    assert!(!profile.detected);
}

#[test]
fn detects_metro_config_and_rn_config() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    let mut f = fs::File::create(root.join("package.json")).unwrap();
    write!(f, r#"{{"dependencies": {{"react-native": "0.73.0"}}}}"#).unwrap();
    fs::File::create(root.join("metro.config.js")).unwrap();
    fs::File::create(root.join("react-native.config.js")).unwrap();

    let profile = detect_react_native_architecture(root);

    assert!(profile.has_metro_config);
    assert!(profile.has_react_native_config);
}

#[test]
fn hermes_conflict_between_platforms_yields_none() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    let mut f = fs::File::create(root.join("package.json")).unwrap();
    write!(f, r#"{{"dependencies": {{"react-native": "0.73.0"}}}}"#).unwrap();

    fs::create_dir(root.join("android")).unwrap();
    let mut gp = fs::File::create(root.join("android/gradle.properties")).unwrap();
    writeln!(gp, "hermesEnabled=true").unwrap();

    fs::create_dir(root.join("ios")).unwrap();
    let mut pf = fs::File::create(root.join("ios/Podfile")).unwrap();
    writeln!(pf, ":hermes_enabled => false").unwrap();

    let profile = detect_react_native_architecture(root);

    assert_eq!(profile.hermes_enabled, None);
    assert!(profile.hermes_mismatch);
}

#[test]
fn detects_architecture_mismatch_and_package_manager() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    let mut f = fs::File::create(root.join("package.json")).unwrap();
    write!(
        f,
        r#"{{"dependencies": {{"react-native": "0.76.0", "expo": "53.0.0"}}}}"#
    )
    .unwrap();
    fs::File::create(root.join("pnpm-lock.yaml")).unwrap();
    fs::create_dir(root.join("android")).unwrap();
    let mut gp = fs::File::create(root.join("android/gradle.properties")).unwrap();
    writeln!(gp, "newArchEnabled=false").unwrap();
    let mut app = fs::File::create(root.join("app.config.js")).unwrap();
    writeln!(
        app,
        "export default {{ expo: {{ newArchEnabled: true }} }};"
    )
    .unwrap();

    let profile = detect_react_native_architecture(root);

    assert_eq!(profile.project_kind, ReactNativeProjectKind::ExpoPrebuild);
    assert!(profile.architecture_mismatch);
    assert_eq!(profile.package_manager, Some("pnpm".to_string()));
}

#[test]
fn gradle_properties_inline_comment_does_not_break_detection() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    let mut f = fs::File::create(root.join("package.json")).unwrap();
    write!(f, r#"{{"dependencies": {{"react-native": "0.73.0"}}}}"#).unwrap();

    fs::create_dir(root.join("android")).unwrap();
    let mut gp = fs::File::create(root.join("android/gradle.properties")).unwrap();
    writeln!(gp, "newArchEnabled=true   # required for Fabric").unwrap();
    writeln!(gp, "hermesEnabled=false # we use JSC").unwrap();

    let profile = detect_react_native_architecture(root);

    assert_eq!(profile.android_new_arch_enabled, Some(true));
    assert_eq!(profile.hermes_enabled, Some(false));
}

#[test]
fn podfile_comment_lines_are_not_parsed_as_config() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    let mut f = fs::File::create(root.join("package.json")).unwrap();
    write!(f, r#"{{"dependencies": {{"react-native": "0.73.0"}}}}"#).unwrap();

    fs::create_dir(root.join("ios")).unwrap();
    let mut pf = fs::File::create(root.join("ios/Podfile")).unwrap();
    // These commented-out lines must not affect detection
    writeln!(pf, "# ENV['RCT_NEW_ARCH_ENABLED'] = '1'").unwrap();
    writeln!(pf, "# :hermes_enabled => true").unwrap();
    // Only the real setting below counts
    writeln!(pf, "ENV['RCT_NEW_ARCH_ENABLED'] = '0'").unwrap();

    let profile = detect_react_native_architecture(root);

    assert_eq!(profile.ios_new_arch_enabled, Some(false));
    assert_eq!(profile.hermes_enabled, None);
}
