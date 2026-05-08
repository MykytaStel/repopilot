use crate::frameworks::detector::extract_version;
use crate::frameworks::react_native::profile::{
    ReactNativeArchitectureProfile, ReactNativeProjectKind,
};
use std::path::Path;

pub fn detect_react_native_architecture(root: &Path) -> ReactNativeArchitectureProfile {
    let pkg_path = root.join("package.json");
    let content = match std::fs::read_to_string(&pkg_path) {
        Ok(c) => c,
        Err(_) => return ReactNativeArchitectureProfile::default(),
    };
    let value: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return ReactNativeArchitectureProfile::default(),
    };

    let mut deps = serde_json::Map::new();
    for key in ["dependencies", "devDependencies"] {
        if let Some(obj) = value.get(key).and_then(|v| v.as_object()) {
            for (k, v) in obj {
                deps.entry(k.clone()).or_insert_with(|| v.clone());
            }
        }
    }

    if !deps.contains_key("react-native") {
        return ReactNativeArchitectureProfile::default();
    }

    let react_native_version = deps
        .get("react-native")
        .and_then(|v| v.as_str())
        .and_then(extract_version);

    let has_expo_dependency = deps.contains_key("expo");
    let has_codegen_config = value.get("codegenConfig").is_some();

    let has_ios = root.join("ios").is_dir();
    let has_android = root.join("android").is_dir();
    let project_kind = project_kind(has_expo_dependency, has_ios, has_android);

    let has_metro_config = ["metro.config.js", "metro.config.cjs", "metro.config.mjs"]
        .iter()
        .any(|name| root.join(name).is_file());

    let has_react_native_config = ["react-native.config.js", "react-native.config.ts"]
        .iter()
        .any(|name| root.join(name).is_file());

    let (has_expo_config, expo_new_arch_enabled) = detect_expo_config(root);

    let gradle_path = root.join("android").join("gradle.properties");
    let android_gradle_properties_found = gradle_path.is_file();
    let (android_new_arch_enabled, android_hermes) = if android_gradle_properties_found {
        match std::fs::read_to_string(&gradle_path) {
            Ok(c) => parse_gradle_properties(&c),
            Err(_) => (None, None),
        }
    } else {
        (None, None)
    };

    let podfile_path = root.join("ios").join("Podfile");
    let ios_podfile_found = podfile_path.is_file();
    let (podfile_new_arch, ios_hermes) = if ios_podfile_found {
        match std::fs::read_to_string(&podfile_path) {
            Ok(c) => parse_podfile(&c),
            Err(_) => (None, None),
        }
    } else {
        (None, None)
    };

    let podfile_properties_path = root.join("ios").join("Podfile.properties.json");
    let ios_podfile_properties_found = podfile_properties_path.is_file();
    let podfile_properties_new_arch = if ios_podfile_properties_found {
        std::fs::read_to_string(&podfile_properties_path)
            .ok()
            .and_then(|content| parse_podfile_properties_json(&content))
    } else {
        None
    };
    let ios_new_arch_enabled = podfile_new_arch.or(podfile_properties_new_arch);

    let architecture_mismatch = has_bool_mismatch([
        android_new_arch_enabled,
        ios_new_arch_enabled,
        expo_new_arch_enabled,
    ]);
    let hermes_mismatch = has_bool_mismatch([android_hermes, ios_hermes, None]);

    // Prefer None when Android and iOS Hermes signals conflict.
    let hermes_enabled = match (android_hermes, ios_hermes) {
        (Some(a), Some(b)) if a == b => Some(a),
        (Some(_), Some(_)) => None,
        (Some(v), None) | (None, Some(v)) => Some(v),
        (None, None) => None,
    };

    ReactNativeArchitectureProfile {
        detected: true,
        react_native_version,
        project_kind,
        has_ios,
        has_android,
        has_metro_config,
        has_react_native_config,
        has_expo_config,
        has_codegen_config,
        expo_new_arch_enabled,
        android_new_arch_enabled,
        ios_new_arch_enabled,
        android_hermes_enabled: android_hermes,
        ios_hermes_enabled: ios_hermes,
        hermes_enabled,
        architecture_mismatch,
        hermes_mismatch,
        package_manager: detect_package_manager(root),
        android_gradle_properties_found,
        ios_podfile_found,
        ios_podfile_properties_found,
    }
}

fn project_kind(
    has_expo_dependency: bool,
    has_ios: bool,
    has_android: bool,
) -> ReactNativeProjectKind {
    match (has_expo_dependency, has_ios || has_android) {
        (true, true) => ReactNativeProjectKind::ExpoPrebuild,
        (true, false) => ReactNativeProjectKind::ExpoManaged,
        (false, true) => ReactNativeProjectKind::Bare,
        (false, false) => ReactNativeProjectKind::Unknown,
    }
}

fn detect_package_manager(root: &Path) -> Option<String> {
    [
        ("pnpm-lock.yaml", "pnpm"),
        ("yarn.lock", "yarn"),
        ("package-lock.json", "npm"),
        ("npm-shrinkwrap.json", "npm"),
        ("bun.lock", "bun"),
        ("bun.lockb", "bun"),
    ]
    .iter()
    .find_map(|(file, manager)| root.join(file).is_file().then(|| (*manager).to_string()))
}

fn detect_expo_config(root: &Path) -> (bool, Option<bool>) {
    let app_json = root.join("app.json");
    if let Ok(content) = std::fs::read_to_string(&app_json) {
        let parsed = serde_json::from_str::<serde_json::Value>(&content)
            .ok()
            .and_then(|value| {
                value
                    .get("expo")
                    .and_then(|expo| expo.get("newArchEnabled"))
                    .and_then(|v| v.as_bool())
            });
        return (true, parsed);
    }

    for name in ["app.config.js", "app.config.ts"] {
        if let Ok(content) = std::fs::read_to_string(root.join(name)) {
            return (true, parse_js_bool_property(&content, "newArchEnabled"));
        }
    }

    (false, None)
}

fn parse_gradle_properties(content: &str) -> (Option<bool>, Option<bool>) {
    let mut new_arch = None;
    let mut hermes = None;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        // Strip trailing inline comment before matching (e.g. "true # required for Fabric")
        let value = value
            .split_once('#')
            .map(|(v, _)| v)
            .unwrap_or(value)
            .trim();
        let parsed = match value {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        };
        match key {
            "newArchEnabled" => new_arch = parsed,
            "hermesEnabled" | "enableHermes" if hermes.is_none() => {
                hermes = parsed;
            }
            _ => {}
        }
    }

    (new_arch, hermes)
}

fn parse_podfile(content: &str) -> (Option<bool>, Option<bool>) {
    let mut new_arch = None;
    let mut hermes = None;

    for line in content.lines() {
        let line = line.trim();
        // Skip Ruby comment lines to avoid parsing commented-out configuration
        if line.starts_with('#') {
            continue;
        }

        if new_arch.is_none() && line.contains("RCT_NEW_ARCH_ENABLED") {
            if line.contains("'1'")
                || line.contains("\"1\"")
                || line.contains("= 1")
                || line.ends_with("=1")
            {
                new_arch = Some(true);
            } else if line.contains("'0'")
                || line.contains("\"0\"")
                || line.contains("= 0")
                || line.ends_with("=0")
            {
                new_arch = Some(false);
            }
        }

        if new_arch.is_none() && line.contains("new_arch_enabled") {
            if line.contains("=> true") {
                new_arch = Some(true);
            } else if line.contains("=> false") {
                new_arch = Some(false);
            }
        }

        if hermes.is_none() && line.contains(":hermes_enabled") {
            if line.contains("=> true") {
                hermes = Some(true);
            } else if line.contains("=> false") {
                hermes = Some(false);
            }
        }
    }

    (new_arch, hermes)
}

fn parse_podfile_properties_json(content: &str) -> Option<bool> {
    serde_json::from_str::<serde_json::Value>(content)
        .ok()
        .and_then(|value| match value.get("newArchEnabled") {
            Some(serde_json::Value::Bool(v)) => Some(*v),
            Some(serde_json::Value::String(v)) if v == "true" => Some(true),
            Some(serde_json::Value::String(v)) if v == "false" => Some(false),
            _ => None,
        })
}

fn parse_js_bool_property(content: &str, property: &str) -> Option<bool> {
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("//") {
            continue;
        }
        if !line.contains(property) {
            continue;
        }
        if line.contains("true") {
            return Some(true);
        }
        if line.contains("false") {
            return Some(false);
        }
    }
    None
}

fn has_bool_mismatch(values: [Option<bool>; 3]) -> bool {
    let mut seen_true = false;
    let mut seen_false = false;
    for value in values.into_iter().flatten() {
        seen_true |= value;
        seen_false |= !value;
    }
    seen_true && seen_false
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
