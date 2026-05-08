use crate::frameworks::detector::extract_version;
use crate::frameworks::react_native::profile::ReactNativeArchitectureProfile;
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

    let has_codegen_config = value.get("codegenConfig").is_some();

    let has_ios = root.join("ios").is_dir();
    let has_android = root.join("android").is_dir();

    let has_metro_config = ["metro.config.js", "metro.config.cjs", "metro.config.mjs"]
        .iter()
        .any(|name| root.join(name).is_file());

    let has_react_native_config = ["react-native.config.js", "react-native.config.ts"]
        .iter()
        .any(|name| root.join(name).is_file());

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
    let (ios_new_arch_enabled, ios_hermes) = if ios_podfile_found {
        match std::fs::read_to_string(&podfile_path) {
            Ok(c) => parse_podfile(&c),
            Err(_) => (None, None),
        }
    } else {
        (None, None)
    };

    // Prefer None when android and iOS hermes signals conflict.
    // TODO: emit a mismatch finding in a future audit when platforms disagree.
    let hermes_enabled = match (android_hermes, ios_hermes) {
        (Some(a), Some(b)) if a == b => Some(a),
        (Some(_), Some(_)) => None,
        (Some(v), None) | (None, Some(v)) => Some(v),
        (None, None) => None,
    };

    ReactNativeArchitectureProfile {
        detected: true,
        react_native_version,
        has_ios,
        has_android,
        has_metro_config,
        has_react_native_config,
        has_codegen_config,
        android_new_arch_enabled,
        ios_new_arch_enabled,
        hermes_enabled,
        android_gradle_properties_found,
        ios_podfile_found,
    }
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
        assert_eq!(profile.hermes_enabled, Some(true));
        assert!(profile.ios_podfile_found);
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
