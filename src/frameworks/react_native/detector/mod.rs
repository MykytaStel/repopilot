use crate::frameworks::detector::extract_version;
use crate::frameworks::react_native::detector::config::{
    detect_expo_config, has_bool_mismatch, parse_gradle_properties, parse_podfile,
    parse_podfile_properties_json,
};
use crate::frameworks::react_native::detector::project::{detect_package_manager, project_kind};
use crate::frameworks::react_native::profile::ReactNativeArchitectureProfile;
use std::path::Path;

mod config;
mod project;

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

#[cfg(test)]
mod tests;
