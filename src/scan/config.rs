use crate::config::defaults::{
    DEFAULT_HUGE_FILE_LINES, DEFAULT_LONG_FUNCTION_LINES, DEFAULT_MAX_DIRECTORY_DEPTH,
    DEFAULT_MAX_DIRECTORY_MODULES, DEFAULT_MAX_FILE_LINES, default_ignored_paths,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanConfig {
    pub ignored_paths: Vec<String>,
    pub large_file_loc_threshold: usize,
    pub huge_file_loc_threshold: usize,
    pub max_directory_modules: usize,
    pub max_directory_depth: usize,
    pub long_function_loc_threshold: usize,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            ignored_paths: default_ignored_paths(),
            large_file_loc_threshold: DEFAULT_MAX_FILE_LINES,
            huge_file_loc_threshold: DEFAULT_HUGE_FILE_LINES,
            max_directory_modules: DEFAULT_MAX_DIRECTORY_MODULES,
            max_directory_depth: DEFAULT_MAX_DIRECTORY_DEPTH,
            long_function_loc_threshold: DEFAULT_LONG_FUNCTION_LINES,
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
