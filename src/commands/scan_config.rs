use repopilot::config::model::RepoPilotConfig;
use repopilot::scan::config::ScanConfig;

#[derive(Debug, Default)]
pub struct ScanConfigOverrides {
    pub max_file_loc: Option<usize>,
    pub max_directory_modules: Option<usize>,
    pub max_directory_depth: Option<usize>,
    pub exclude_patterns: Vec<String>,
    pub include_low_signal: bool,
    pub max_file_size: Option<u64>,
    pub max_files: Option<usize>,
}

pub fn build_scan_config(
    repo_config: &RepoPilotConfig,
    overrides: ScanConfigOverrides,
) -> ScanConfig {
    let mut config = repo_config.to_scan_config();

    if let Some(threshold) = overrides.max_file_loc {
        config = config.with_large_file_loc_threshold(threshold);
    }

    if let Some(modules) = overrides.max_directory_modules {
        config.max_directory_modules = modules;
    }

    if let Some(depth) = overrides.max_directory_depth {
        config.max_directory_depth = depth;
    }

    config.exclude_patterns = overrides.exclude_patterns;
    config.include_low_signal = overrides.include_low_signal;
    if let Some(bytes) = overrides.max_file_size {
        config.max_file_bytes = bytes;
    }
    config.max_files = overrides.max_files;

    config
}
