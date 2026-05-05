#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanConfig {
    pub large_file_loc_threshold: usize,
    pub huge_file_loc_threshold: usize,
    pub max_directory_modules: usize,
    pub max_directory_depth: usize,
    pub long_function_loc_threshold: usize,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            large_file_loc_threshold: 300,
            huge_file_loc_threshold: 1000,
            max_directory_modules: 20,
            max_directory_depth: 5,
            long_function_loc_threshold: 50,
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
