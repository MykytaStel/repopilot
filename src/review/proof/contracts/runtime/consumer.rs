use crate::review::diff::ChangedFile;
use ignore::WalkBuilder;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use super::super::ContractConfidence;

const MAX_SOURCE_BYTES: u64 = 1_048_576;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ConsumerMatch {
    pub(super) path: String,
    pub(super) confidence: ContractConfidence,
    pub(super) evidence: String,
}

pub(super) fn find(
    repo_root: Option<&Path>,
    changed_files: &[ChangedFile],
    key: &str,
) -> Option<ConsumerMatch> {
    let changed = changed_matches(changed_files, key);
    if !changed.is_empty() {
        return summarize(changed, "changed consumer");
    }

    let root = repo_root?;
    let changed_paths = changed_files
        .iter()
        .map(|file| file.path_string())
        .collect::<BTreeSet<_>>();
    let mut matches = Vec::new();
    let mut paths = WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .build()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_some_and(|kind| kind.is_file()))
        .map(|entry| entry.into_path())
        .filter(|path| is_candidate_path(path, root, &changed_paths))
        .collect::<Vec<_>>();
    paths.sort();

    for path in paths {
        let Ok(metadata) = fs::metadata(&path) else {
            continue;
        };
        if metadata.len() > MAX_SOURCE_BYTES {
            continue;
        }
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        for (line_index, line) in content.lines().enumerate() {
            if line_uses_key(line, key) {
                matches.push((relative_path(root, &path), line_index + 1));
            }
        }
    }
    summarize(matches, "unchanged consumer")
}

fn changed_matches(changed_files: &[ChangedFile], key: &str) -> Vec<(String, usize)> {
    let mut matches = Vec::new();
    for file in changed_files {
        if super::is_runtime_config_path(&file.path_string()) {
            continue;
        }
        for hunk in &file.hunks {
            let new_start = hunk.new_range.map(|range| range.start).unwrap_or(1);
            for (offset, line) in hunk.added_lines.iter().enumerate() {
                if line_uses_key(line, key) {
                    matches.push((file.path_string(), new_start + offset));
                }
            }
            let old_start = hunk.old_range.map(|range| range.start).unwrap_or(1);
            for (offset, line) in hunk.removed_lines.iter().enumerate() {
                if line_uses_key(line, key) {
                    matches.push((file.path_string(), old_start + offset));
                }
            }
        }
    }
    matches.sort();
    matches
}

fn summarize(matches: Vec<(String, usize)>, label: &str) -> Option<ConsumerMatch> {
    let first = matches.first()?.clone();
    let confidence = (matches.len() == 1).then_some(ContractConfidence::High);
    let evidence = if matches.len() == 1 {
        format!("Found {label} at `{}:{}`.", first.0, first.1)
    } else {
        format!(
            "Found multiple consumers ({}); exact runtime consumer is ambiguous.",
            matches
                .iter()
                .map(|(path, line)| format!("`{path}:{line}`"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    Some(ConsumerMatch {
        path: first.0,
        confidence: confidence.unwrap_or(ContractConfidence::Limited),
        evidence,
    })
}

fn is_candidate_path(path: &Path, root: &Path, changed_paths: &BTreeSet<String>) -> bool {
    let relative = relative_path(root, path);
    if changed_paths.contains(&relative) || super::is_runtime_config_path(&relative) {
        return false;
    }
    if relative.split('/').any(|part| {
        matches!(
            part,
            ".git"
                | "target"
                | "node_modules"
                | "vendor"
                | "docs"
                | "examples"
                | "fixtures"
                | "tests"
                | "test"
                | "__tests__"
        )
    }) {
        return false;
    }
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension,
                "cjs"
                    | "cs"
                    | "go"
                    | "java"
                    | "js"
                    | "json"
                    | "jsx"
                    | "kt"
                    | "kts"
                    | "py"
                    | "rs"
                    | "sh"
                    | "ts"
                    | "tsx"
                    | "toml"
                    | "yaml"
                    | "yml"
            )
        })
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn line_uses_key(line: &str, key: &str) -> bool {
    [
        format!("process.env.{key}"),
        format!("os.getenv(\"{key}\""),
        format!("getenv(\"{key}\""),
        format!("System.getenv(\"{key}\""),
        format!("config.{key}"),
        format!("${{{key}}}"),
    ]
    .iter()
    .any(|needle| {
        line.match_indices(needle).any(|(start, _)| {
            let end = start + needle.len();
            line.as_bytes()
                .get(end)
                .is_none_or(|byte| !byte.is_ascii_alphanumeric() && *byte != b'_')
        })
    })
}
