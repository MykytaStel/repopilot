use crate::audits::traits::ProjectAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::frameworks::ReactNativeArchitectureProfile;
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;
use std::path::PathBuf;

pub struct ReactNativeOldArchAudit;

impl ProjectAudit for ReactNativeOldArchAudit {
    fn audit(&self, facts: &ScanFacts, _config: &ScanConfig) -> Vec<Finding> {
        let Some(profile) = &facts.react_native else {
            return vec![];
        };

        if any_new_arch_enabled(profile) && !any_new_arch_disabled(profile) {
            return vec![];
        }

        let (evidence_path, snippet) = profile_config_evidence(&facts.root_path, profile);

        vec![Finding {
            id: String::new(),
            rule_id: "framework.react-native.old-architecture".to_string(),
        recommendation: Finding::recommendation_for_rule_id("framework.react-native.old-architecture"),
            title: "React Native New Architecture is not enabled".to_string(),
            description: concat!(
                "This project does not have `newArchEnabled: true` in its app.json / app.config.js / react-native.config.js. ",
                "The New Architecture (Fabric renderer + TurboModules) eliminates the asynchronous JS bridge, ",
                "delivers faster UI updates, and is required by an increasing number of third-party libraries. ",
                "Enable it by setting `\"newArchEnabled\": true` in your app.json `expo` block or in react-native.config.js."
            ).to_string(),
            category: FindingCategory::Framework,
            severity: Severity::Medium,
            confidence: Default::default(),
            evidence: vec![Evidence {
                path: evidence_path,
                line_start: 1,
                line_end: None,
                snippet,
            }],
            workspace_package: None,
            docs_url: Some("https://reactnative.dev/docs/new-architecture-intro".to_string()),
        }]
    }
}

pub struct ReactNativeArchitectureMismatchAudit;

impl ProjectAudit for ReactNativeArchitectureMismatchAudit {
    fn audit(&self, facts: &ScanFacts, _config: &ScanConfig) -> Vec<Finding> {
        let Some(profile) = &facts.react_native else {
            return vec![];
        };
        if !profile.architecture_mismatch {
            return vec![];
        }

        vec![Finding {
            id: String::new(),
            rule_id: "framework.react-native.architecture-mismatch".to_string(),
        recommendation: Finding::recommendation_for_rule_id("framework.react-native.architecture-mismatch"),
            title: "React Native New Architecture settings differ by platform".to_string(),
            description: concat!(
                "Android, iOS, or Expo configuration disagree about React Native New Architecture. ",
                "Align these settings so local builds, CI builds, and release builds run the same runtime."
            )
            .to_string(),
            category: FindingCategory::Framework,
            severity: Severity::High,
            confidence: Default::default(),
            evidence: vec![Evidence {
                path: profile_config_evidence(&facts.root_path, profile).0,
                line_start: 1,
                line_end: None,
                snippet: format!(
                    "android={}; ios={}; expo={}",
                    format_bool(profile.android_new_arch_enabled),
                    format_bool(profile.ios_new_arch_enabled),
                    format_bool(profile.expo_new_arch_enabled)
                ),
            }],
            workspace_package: None,
            docs_url: None,
        }]
    }
}

pub struct HermesMismatchAudit;

impl ProjectAudit for HermesMismatchAudit {
    fn audit(&self, facts: &ScanFacts, _config: &ScanConfig) -> Vec<Finding> {
        let Some(profile) = &facts.react_native else {
            return vec![];
        };
        if !profile.hermes_mismatch {
            return vec![];
        }

        vec![Finding {
            id: String::new(),
            rule_id: "framework.react-native.hermes-mismatch".to_string(),
        recommendation: Finding::recommendation_for_rule_id("framework.react-native.hermes-mismatch"),
            title: "Hermes settings differ between Android and iOS".to_string(),
            description: concat!(
                "Hermes is configured differently across platforms. ",
                "Align Android and iOS Hermes settings to avoid platform-specific runtime and performance behavior."
            )
            .to_string(),
            category: FindingCategory::Framework,
            severity: Severity::Medium,
            confidence: Default::default(),
            evidence: vec![Evidence {
                path: facts.root_path.clone(),
                line_start: 1,
                line_end: None,
                snippet: format!(
                    "android={}; ios={}",
                    format_bool(profile.android_hermes_enabled),
                    format_bool(profile.ios_hermes_enabled)
                ),
            }],
            workspace_package: None,
            docs_url: None,
        }]
    }
}

fn any_new_arch_enabled(profile: &ReactNativeArchitectureProfile) -> bool {
    [
        profile.android_new_arch_enabled,
        profile.ios_new_arch_enabled,
        profile.expo_new_arch_enabled,
    ]
    .into_iter()
    .any(|value| value == Some(true))
}

fn any_new_arch_disabled(profile: &ReactNativeArchitectureProfile) -> bool {
    [
        profile.android_new_arch_enabled,
        profile.ios_new_arch_enabled,
        profile.expo_new_arch_enabled,
    ]
    .into_iter()
    .any(|value| value == Some(false))
}

pub(super) fn profile_config_evidence(
    root: &std::path::Path,
    profile: &ReactNativeArchitectureProfile,
) -> (PathBuf, String) {
    if profile.android_gradle_properties_found {
        return (
            root.join("android/gradle.properties"),
            "newArchEnabled is not enabled for Android".to_string(),
        );
    }
    if profile.ios_podfile_properties_found {
        return (
            root.join("ios/Podfile.properties.json"),
            "newArchEnabled is not enabled for iOS".to_string(),
        );
    }
    if profile.ios_podfile_found {
        return (
            root.join("ios/Podfile"),
            "new_arch_enabled is not enabled for iOS".to_string(),
        );
    }
    for name in ["app.json", "app.config.js", "app.config.ts"] {
        if root.join(name).exists() {
            return (
                root.join(name),
                format!("newArchEnabled not set to true in {name}"),
            );
        }
    }

    (
        root.to_path_buf(),
        "No React Native New Architecture configuration found".to_string(),
    )
}

pub(super) fn format_bool(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "enabled",
        Some(false) => "disabled",
        None => "unknown",
    }
}
