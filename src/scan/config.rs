use crate::config::defaults::{
    DEFAULT_COMPLEX_FUNCTION_THRESHOLD, DEFAULT_COMPLEXITY_HIGH_THRESHOLD,
    DEFAULT_COMPLEXITY_MEDIUM_THRESHOLD, DEFAULT_HUGE_FILE_LINES,
    DEFAULT_INSTABILITY_HUB_MIN_FAN_IN, DEFAULT_INSTABILITY_HUB_MIN_INSTABILITY_PCT,
    DEFAULT_LONG_FUNCTION_LINES, DEFAULT_MAX_CONTROL_FLOW_DEPTH, DEFAULT_MAX_DIRECTORY_DEPTH,
    DEFAULT_MAX_DIRECTORY_MODULES, DEFAULT_MAX_FAN_OUT, DEFAULT_MAX_FILE_BYTES,
    DEFAULT_MAX_FILE_LINES, default_ignored_paths,
};
use crate::findings::types::Severity;
use serde::Serialize;

/// One declared architectural layer (opt-in `[[architecture.layers]]`). Layers
/// are ordered from highest-level to lowest-level; a module may depend on layers
/// listed at or below its own, never above. Empty list = the layer rule is off.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct LayerSpec {
    pub name: String,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScanConfig {
    pub ignored_paths: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub max_file_bytes: u64,
    pub include_low_signal: bool,
    pub max_files: Option<usize>,
    pub detect_missing_tests: bool,
    pub detect_secret_like_names: bool,
    pub large_file_loc_threshold: usize,
    pub huge_file_loc_threshold: usize,
    pub max_directory_modules: usize,
    pub max_directory_depth: usize,
    pub long_function_loc_threshold: usize,
    pub complexity_medium_threshold: usize,
    pub complexity_high_threshold: usize,
    pub complex_function_threshold: usize,
    pub max_fan_out: usize,
    pub instability_hub_min_fan_in: usize,
    pub instability_hub_min_instability_pct: usize,
    pub max_control_flow_depth: usize,
    pub module_mappings: std::collections::BTreeMap<String, Vec<String>>,
    /// Ordered, user-declared architectural layers (opt-in
    /// `[[architecture.layers]]`). Empty = `architecture.layer-violation` is off.
    pub architecture_layers: Vec<LayerSpec>,
    /// Glob roots whose immediate children are independent packages/features
    /// (opt-in `[architecture] package_roots`, e.g. `packages/*`). Empty =
    /// `architecture.package-boundary-violation` is off.
    pub package_roots: Vec<String>,
    /// Rule ids whose findings are dropped (validated `[rules] disable`).
    pub disabled_rules: std::collections::BTreeSet<String>,
    /// Absolute per-rule severity overrides (validated `[rules.severity_overrides]`).
    pub severity_overrides: std::collections::BTreeMap<String, Severity>,
    /// Invalid `[rules]` entries (unknown rule id, bad severity label), kept so
    /// the scan surfaces them as diagnostics instead of failing silently.
    pub rule_config_problems: Vec<String>,
}

impl Default for ScanConfig {
    fn default() -> Self {
        let mut module_mappings = std::collections::BTreeMap::new();
        module_mappings.insert(
            "feature".to_string(),
            vec![
                "**/features/**".to_string(),
                "**/apps/**".to_string(),
                "**/pages/**".to_string(),
            ],
        );
        module_mappings.insert(
            "shared".to_string(),
            vec![
                "**/shared/**".to_string(),
                "**/utils/**".to_string(),
                "**/common/**".to_string(),
                "**/helpers/**".to_string(),
            ],
        );
        module_mappings.insert(
            "infrastructure".to_string(),
            vec![
                "**/infra/**".to_string(),
                "**/infrastructure/**".to_string(),
                "**/db/**".to_string(),
                "**/database/**".to_string(),
                "**/persistence/**".to_string(),
                "**/gateways/**".to_string(),
            ],
        );
        module_mappings.insert(
            "domain".to_string(),
            vec![
                "**/domain/**".to_string(),
                "**/domains/**".to_string(),
                "**/model/**".to_string(),
                "**/models/**".to_string(),
                "**/entities/**".to_string(),
            ],
        );
        module_mappings.insert(
            "ui".to_string(),
            vec![
                "**/ui/**".to_string(),
                "**/components/**".to_string(),
                "**/views/**".to_string(),
                "**/screens/**".to_string(),
            ],
        );
        module_mappings.insert(
            "cli".to_string(),
            vec![
                "**/cli/**".to_string(),
                "**/bin/**".to_string(),
                "**/commands/**".to_string(),
            ],
        );

        Self {
            ignored_paths: default_ignored_paths(),
            exclude_patterns: Vec::new(),
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
            include_low_signal: false,
            max_files: None,
            detect_missing_tests: true,
            detect_secret_like_names: true,
            large_file_loc_threshold: DEFAULT_MAX_FILE_LINES,
            huge_file_loc_threshold: DEFAULT_HUGE_FILE_LINES,
            max_directory_modules: DEFAULT_MAX_DIRECTORY_MODULES,
            max_directory_depth: DEFAULT_MAX_DIRECTORY_DEPTH,
            long_function_loc_threshold: DEFAULT_LONG_FUNCTION_LINES,
            complexity_medium_threshold: DEFAULT_COMPLEXITY_MEDIUM_THRESHOLD,
            complexity_high_threshold: DEFAULT_COMPLEXITY_HIGH_THRESHOLD,
            complex_function_threshold: DEFAULT_COMPLEX_FUNCTION_THRESHOLD,
            max_fan_out: DEFAULT_MAX_FAN_OUT,
            instability_hub_min_fan_in: DEFAULT_INSTABILITY_HUB_MIN_FAN_IN,
            instability_hub_min_instability_pct: DEFAULT_INSTABILITY_HUB_MIN_INSTABILITY_PCT,
            max_control_flow_depth: DEFAULT_MAX_CONTROL_FLOW_DEPTH,
            module_mappings,
            architecture_layers: Vec::new(),
            package_roots: Vec::new(),
            disabled_rules: std::collections::BTreeSet::new(),
            severity_overrides: std::collections::BTreeMap::new(),
            rule_config_problems: Vec::new(),
        }
    }
}

impl ScanConfig {
    pub fn with_large_file_loc_threshold(mut self, threshold: usize) -> Self {
        self.large_file_loc_threshold = threshold;

        if self.huge_file_loc_threshold <= threshold {
            self.huge_file_loc_threshold = threshold.saturating_mul(3).max(threshold + 1);
        }

        self
    }
}
