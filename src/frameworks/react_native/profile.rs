use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReactNativeArchitectureProfile {
    #[serde(default)]
    pub detected: bool,
    #[serde(default)]
    pub react_native_version: Option<String>,
    #[serde(default)]
    pub project_kind: ReactNativeProjectKind,

    #[serde(default)]
    pub has_ios: bool,
    #[serde(default)]
    pub has_android: bool,

    #[serde(default)]
    pub has_metro_config: bool,
    #[serde(default)]
    pub has_react_native_config: bool,
    #[serde(default)]
    pub has_expo_config: bool,
    #[serde(default)]
    pub has_codegen_config: bool,

    #[serde(default)]
    pub expo_new_arch_enabled: Option<bool>,
    #[serde(default)]
    pub android_new_arch_enabled: Option<bool>,
    #[serde(default)]
    pub ios_new_arch_enabled: Option<bool>,

    #[serde(default)]
    pub android_hermes_enabled: Option<bool>,
    #[serde(default)]
    pub ios_hermes_enabled: Option<bool>,
    #[serde(default)]
    pub hermes_enabled: Option<bool>,

    #[serde(default)]
    pub architecture_mismatch: bool,
    #[serde(default)]
    pub hermes_mismatch: bool,
    #[serde(default)]
    pub package_manager: Option<String>,

    #[serde(default)]
    pub android_gradle_properties_found: bool,
    #[serde(default)]
    pub ios_podfile_found: bool,
    #[serde(default)]
    pub ios_podfile_properties_found: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReactNativeProjectKind {
    Bare,
    ExpoManaged,
    ExpoPrebuild,
    #[default]
    Unknown,
}
