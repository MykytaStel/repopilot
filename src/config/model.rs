use crate::config::defaults::{
    DEFAULT_COMPLEXITY_HIGH_THRESHOLD, DEFAULT_COMPLEXITY_MEDIUM_THRESHOLD,
    DEFAULT_HUGE_FILE_LINES, DEFAULT_INSTABILITY_HUB_MIN_FAN_IN,
    DEFAULT_INSTABILITY_HUB_MIN_INSTABILITY_PCT, DEFAULT_LONG_FUNCTION_LINES,
    DEFAULT_MAX_CONTROL_FLOW_DEPTH, DEFAULT_MAX_DIRECTORY_DEPTH, DEFAULT_MAX_DIRECTORY_MODULES,
    DEFAULT_MAX_FAN_OUT, DEFAULT_MAX_FILE_BYTES, DEFAULT_MAX_FILE_LINES, default_ignored_paths,
};
use crate::output::OutputFormat;
use crate::scan::config::ScanConfig;
use serde::Deserialize;

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct RepoPilotConfig {
    pub scan: ScanSection,
    pub architecture: ArchitectureSection,
    pub code_quality: CodeQualitySection,
    pub testing: TestingSection,
    pub security: SecuritySection,
    pub security_boundary: SecurityBoundarySection,
    pub behavioral: BehavioralSection,
    pub algorithmic: AlgorithmicSection,
    pub output: OutputSection,
}

impl RepoPilotConfig {
    pub fn to_scan_config(&self) -> ScanConfig {
        let mut config =
            ScanConfig::default().with_large_file_loc_threshold(self.architecture.max_file_lines);
        config.ignored_paths = self.scan.ignore.clone();
        config.max_file_bytes = self.scan.max_file_bytes;
        config.detect_missing_tests = self.testing.detect_missing_tests;
        config.detect_secret_like_names = self.security.detect_secret_like_names;
        config.huge_file_loc_threshold = self.architecture.huge_file_lines;
        if config.huge_file_loc_threshold <= config.large_file_loc_threshold {
            config.huge_file_loc_threshold = config
                .large_file_loc_threshold
                .saturating_mul(3)
                .max(config.large_file_loc_threshold + 1);
        }
        config.max_directory_modules = self.architecture.max_directory_modules;
        config.max_directory_depth = self.architecture.max_directory_depth;
        config.long_function_loc_threshold = self.architecture.max_function_lines;
        config.max_fan_out = self.architecture.max_fan_out;
        config.instability_hub_min_fan_in = self.architecture.instability_hub_min_fan_in;
        config.instability_hub_min_instability_pct =
            self.architecture.instability_hub_min_instability_pct;
        config.complexity_medium_threshold = self.code_quality.complexity_medium_threshold;
        config.complexity_high_threshold = self.code_quality.complexity_high_threshold;
        config.max_control_flow_depth = self.code_quality.max_control_flow_depth;
        if !self.architecture.module_mappings.is_empty() {
            config.module_mappings = self.architecture.module_mappings.clone();
        }
        config
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct ScanSection {
    #[serde(default = "default_ignored_paths")]
    pub ignore: Vec<String>,
    pub max_file_bytes: u64,
}

impl Default for ScanSection {
    fn default() -> Self {
        Self {
            ignore: default_ignored_paths(),
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct ArchitectureSection {
    pub max_file_lines: usize,
    pub huge_file_lines: usize,
    pub max_directory_modules: usize,
    pub max_directory_depth: usize,
    pub max_function_lines: usize,
    pub max_fan_out: usize,
    pub instability_hub_min_fan_in: usize,
    pub instability_hub_min_instability_pct: usize,
    pub module_mappings: std::collections::BTreeMap<String, Vec<String>>,
}

impl Default for ArchitectureSection {
    fn default() -> Self {
        Self {
            max_file_lines: DEFAULT_MAX_FILE_LINES,
            huge_file_lines: DEFAULT_HUGE_FILE_LINES,
            max_directory_modules: DEFAULT_MAX_DIRECTORY_MODULES,
            max_directory_depth: DEFAULT_MAX_DIRECTORY_DEPTH,
            max_function_lines: DEFAULT_LONG_FUNCTION_LINES,
            max_fan_out: DEFAULT_MAX_FAN_OUT,
            instability_hub_min_fan_in: DEFAULT_INSTABILITY_HUB_MIN_FAN_IN,
            instability_hub_min_instability_pct: DEFAULT_INSTABILITY_HUB_MIN_INSTABILITY_PCT,
            module_mappings: std::collections::BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct CodeQualitySection {
    pub complexity_medium_threshold: usize,
    pub complexity_high_threshold: usize,
    pub max_control_flow_depth: usize,
}

impl Default for CodeQualitySection {
    fn default() -> Self {
        Self {
            complexity_medium_threshold: DEFAULT_COMPLEXITY_MEDIUM_THRESHOLD,
            complexity_high_threshold: DEFAULT_COMPLEXITY_HIGH_THRESHOLD,
            max_control_flow_depth: DEFAULT_MAX_CONTROL_FLOW_DEPTH,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct TestingSection {
    pub detect_missing_tests: bool,
}

impl Default for TestingSection {
    fn default() -> Self {
        Self {
            detect_missing_tests: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct SecuritySection {
    pub detect_secret_like_names: bool,
}

impl Default for SecuritySection {
    fn default() -> Self {
        Self {
            detect_secret_like_names: true,
        }
    }
}

/// Configures the `review` security-boundary change signals. The detector flags
/// (it does not prove) when a change touches who-can-do-what or how the app
/// ships. See `src/review/signals.rs`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct SecurityBoundarySection {
    /// Whether to surface boundary signals at all. Defaults to enabled.
    pub enabled: bool,
    /// Extra glob patterns (matched against repo-relative paths) to flag as
    /// boundary changes, in addition to the built-in defaults. Reported under
    /// the `custom` category.
    pub extra_patterns: Vec<String>,
}

impl Default for SecurityBoundarySection {
    fn default() -> Self {
        Self {
            enabled: true,
            extra_patterns: Vec::new(),
        }
    }
}

/// Configures the `review` behavioral change signals (what a change *does* —
/// network calls, subprocess/exec, env vars, migrations, removed error handling
/// or tests, …). Detected from the changed lines; flags, does not prove.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct BehavioralSection {
    /// Whether to surface behavioral signals at all. Defaults to enabled.
    pub enabled: bool,
}

impl Default for BehavioralSection {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// Configures the `review` algorithmic change signals (structural deltas in the
/// functions a change touched — nesting, nested loops, growth, recursion). The
/// highest-noise family; reports the structural fact, never a verdict.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct AlgorithmicSection {
    /// Whether to surface algorithmic signals at all. Defaults to enabled.
    pub enabled: bool,
}

impl Default for AlgorithmicSection {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct OutputSection {
    pub default_format: OutputFormat,
}

impl Default for OutputSection {
    fn default() -> Self {
        Self {
            default_format: OutputFormat::Console,
        }
    }
}
