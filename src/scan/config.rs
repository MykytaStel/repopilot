use crate::config::defaults::{
    DEFAULT_COMPLEXITY_HIGH_THRESHOLD, DEFAULT_COMPLEXITY_MEDIUM_THRESHOLD,
    DEFAULT_HUGE_FILE_LINES, DEFAULT_INSTABILITY_HUB_MIN_FAN_IN,
    DEFAULT_INSTABILITY_HUB_MIN_INSTABILITY_PCT, DEFAULT_LONG_FUNCTION_LINES,
    DEFAULT_MAX_DIRECTORY_DEPTH, DEFAULT_MAX_DIRECTORY_MODULES, DEFAULT_MAX_FAN_OUT,
    DEFAULT_MAX_FILE_BYTES, DEFAULT_MAX_FILE_LINES, default_ignored_paths,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanConfig {
    pub ignored_paths: Vec<String>,
    pub max_file_bytes: u64,
    pub large_file_loc_threshold: usize,
    pub huge_file_loc_threshold: usize,
    pub max_directory_modules: usize,
    pub max_directory_depth: usize,
    pub long_function_loc_threshold: usize,
    pub complexity_medium_threshold: usize,
    pub complexity_high_threshold: usize,
    pub max_fan_out: usize,
    pub instability_hub_min_fan_in: usize,
    pub instability_hub_min_instability_pct: usize,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            ignored_paths: default_ignored_paths(),
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
            large_file_loc_threshold: DEFAULT_MAX_FILE_LINES,
            huge_file_loc_threshold: DEFAULT_HUGE_FILE_LINES,
            max_directory_modules: DEFAULT_MAX_DIRECTORY_MODULES,
            max_directory_depth: DEFAULT_MAX_DIRECTORY_DEPTH,
            long_function_loc_threshold: DEFAULT_LONG_FUNCTION_LINES,
            complexity_medium_threshold: DEFAULT_COMPLEXITY_MEDIUM_THRESHOLD,
            complexity_high_threshold: DEFAULT_COMPLEXITY_HIGH_THRESHOLD,
            max_fan_out: DEFAULT_MAX_FAN_OUT,
            instability_hub_min_fan_in: DEFAULT_INSTABILITY_HUB_MIN_FAN_IN,
            instability_hub_min_instability_pct: DEFAULT_INSTABILITY_HUB_MIN_INSTABILITY_PCT,
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
