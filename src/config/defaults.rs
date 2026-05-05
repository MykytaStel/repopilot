pub const CONFIG_FILE_NAME: &str = "repopilot.toml";

pub const DEFAULT_IGNORED_PATHS: &[&str] = &[
    ".git",
    ".github",
    "target",
    "node_modules",
    "dist",
    "build",
    ".next",
    "coverage",
];

pub const DEFAULT_MAX_FILE_LINES: usize = 300;
pub const DEFAULT_HUGE_FILE_LINES: usize = 1000;
pub const DEFAULT_MAX_DIRECTORY_MODULES: usize = 20;
pub const DEFAULT_MAX_DIRECTORY_DEPTH: usize = 5;
pub const DEFAULT_LONG_FUNCTION_LINES: usize = 50;

pub fn default_ignored_paths() -> Vec<String> {
    DEFAULT_IGNORED_PATHS
        .iter()
        .map(|path| (*path).to_string())
        .collect()
}
