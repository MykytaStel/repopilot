//! Framework rules report per file. A changed scan reports them for the
//! changed files, like every other per-file rule; a full scan for every file.

use super::super::full::scan_path_with_config;
use super::super::scan_resolved_changed_with_config;
use crate::review::diff::{ChangeStatus, ChangedFile};
use crate::scan::config::ScanConfig;
use std::fs;
use std::path::Path;

const LOGGING: &str =
    "export function run(value: string): string {\n  console.log(value);\n  return value;\n}\n";

fn write(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent directory")).expect("create parent");
    fs::write(path, content).expect("write source file");
}

fn console_log_paths(summary: &crate::scan::types::ScanSummary) -> Vec<String> {
    let mut paths = summary
        .artifacts
        .findings
        .iter()
        .filter(|finding| finding.rule_id == "framework.js.console-log")
        .map(|finding| {
            let path = &finding.evidence[0].path;
            path.strip_prefix(&summary.root_path)
                .unwrap_or(path)
                .to_string_lossy()
                .trim_start_matches("./")
                .to_string()
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

#[test]
fn a_changed_scan_reports_framework_rules_for_changed_files_only() {
    let temp = tempfile::tempdir().expect("temporary repository");
    let root = temp.path();
    write(
        root,
        "package.json",
        "{\"name\": \"app\", \"dependencies\": {\"react\": \"18.0.0\"}}\n",
    );
    write(root, "src/changed.ts", LOGGING);
    write(root, "src/untouched.ts", LOGGING);
    let config = ScanConfig::default();

    let full = scan_path_with_config(root, &config).expect("full scan");
    assert_eq!(
        console_log_paths(&full),
        vec!["src/changed.ts".to_string(), "src/untouched.ts".to_string()]
    );

    let changed = vec![ChangedFile {
        path: "src/changed.ts".into(),
        status: ChangeStatus::Modified,
        ranges: Vec::new(),
        hunks: Vec::new(),
    }];
    let review =
        scan_resolved_changed_with_config(root, &config, root.to_path_buf(), changed, Some("HEAD"))
            .expect("changed scan");
    assert_eq!(
        console_log_paths(&review),
        vec!["src/changed.ts".to_string()]
    );
}
