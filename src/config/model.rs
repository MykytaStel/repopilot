use crate::config::defaults::{
    DEFAULT_HUGE_FILE_LINES, DEFAULT_LONG_FUNCTION_LINES, DEFAULT_MAX_DIRECTORY_DEPTH,
    DEFAULT_MAX_DIRECTORY_MODULES, DEFAULT_MAX_FILE_LINES, default_ignored_paths,
};
use crate::output::OutputFormat;
use crate::scan::config::ScanConfig;
use serde::Deserialize;

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct RepoPilotConfig {
    pub scan: ScanSection,
    pub architecture: ArchitectureSection,
    pub testing: TestingSection,
    pub security: SecuritySection,
    pub output: OutputSection,
}

impl RepoPilotConfig {
    pub fn to_scan_config(&self) -> ScanConfig {
        let mut config =
            ScanConfig::default().with_large_file_loc_threshold(self.architecture.max_file_lines);
        config.ignored_paths = self.scan.ignore.clone();
        config.huge_file_loc_threshold = self.architecture.huge_file_lines;
        if config.huge_file_loc_threshold <= config.large_file_loc_threshold {
            config.huge_file_loc_threshold = config
                .large_file_loc_threshold
                .saturating_mul(3)
                .max(config.large_file_loc_threshold + 1);
        }
        config.max_directory_modules = self.architecture.max_directory_modules;
        config.max_directory_depth = self.architecture.max_directory_depth;
        config.long_function_loc_threshold = DEFAULT_LONG_FUNCTION_LINES;
        config
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct ScanSection {
    #[serde(default = "default_ignored_paths")]
    pub ignore: Vec<String>,
}

impl Default for ScanSection {
    fn default() -> Self {
        Self {
            ignore: default_ignored_paths(),
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
    pub detect_empty_directories: bool,
    pub detect_suspicious_names: bool,
    pub detect_large_files: bool,
}

impl Default for ArchitectureSection {
    fn default() -> Self {
        Self {
            max_file_lines: DEFAULT_MAX_FILE_LINES,
            huge_file_lines: DEFAULT_HUGE_FILE_LINES,
            max_directory_modules: DEFAULT_MAX_DIRECTORY_MODULES,
            max_directory_depth: DEFAULT_MAX_DIRECTORY_DEPTH,
            detect_empty_directories: true,
            detect_suspicious_names: true,
            detect_large_files: true,
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
