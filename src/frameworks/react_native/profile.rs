use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReactNativeArchitectureProfile {
    pub detected: bool,
    pub react_native_version: Option<String>,

    pub has_ios: bool,
    pub has_android: bool,

    pub has_metro_config: bool,
    pub has_react_native_config: bool,
    pub has_codegen_config: bool,

    pub android_new_arch_enabled: Option<bool>,
    pub ios_new_arch_enabled: Option<bool>,

    pub hermes_enabled: Option<bool>,

    pub android_gradle_properties_found: bool,
    pub ios_podfile_found: bool,
}
