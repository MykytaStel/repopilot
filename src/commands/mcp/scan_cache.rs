//! Cross-session disk cache for the `repopilot_scan` tool. A repeated scan of an
//! unchanged tree is a cache hit, so an agent's edit-rescan loop does not
//! re-audit from scratch every session.
//!
//! Correctness over speed: the key includes a working-tree fingerprint, config
//! and feedback control files, tool/schema versions, and the resolved changed
//! scan base commit when applicable. If any required cache input is uncertain,
//! the caller gets `None` and runs a fresh scan.

mod git;
mod storage;

use repopilot::config::loader::discover_config_path;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const SCHEMA: &str = "mcp-scan-cache-v1";
const FEEDBACK_PATH: &str = ".repopilot/feedback.yml";
const REPOPILOT_IGNORE_FILENAME: &str = ".repopilotignore";

/// The cache key for a scan call, or `None` when cache correctness cannot be
/// established. `None` means "run a fresh scan and do not read/write disk cache."
pub(super) fn cache_key(path: &Path, arguments: &Value) -> Option<String> {
    cache_key_with_metadata(path, arguments, SCHEMA, env!("CARGO_PKG_VERSION"))
}

fn cache_key_with_metadata(
    path: &Path,
    arguments: &Value,
    schema: &str,
    version: &str,
) -> Option<String> {
    let repo_root = git::git_root(path)?;
    storage::cache_dir(path)?;

    let tree = git::working_tree_fingerprint(&repo_root)?;
    let changed_base = changed_scope_base_blob(&repo_root, arguments)?;
    let ignore_sources = ignore_sources_blob(path, &repo_root)?;

    let mut hasher = Sha256::new();
    hasher.update(schema.as_bytes());
    hasher.update(b"\nversion:");
    hasher.update(version.as_bytes());
    hasher.update(b"\nargs:");
    hasher.update(canonical_json(arguments).as_bytes());
    hasher.update(b"\nchanged-base:");
    hasher.update(changed_base.as_bytes());
    hasher.update(b"\nconfig:");
    hasher.update(config_blob(path, arguments).as_bytes());
    hasher.update(b"\nfeedback:");
    hasher.update(path_blob(path.join(FEEDBACK_PATH)).as_bytes());
    hasher.update(b"\nrepopilotignore:");
    hasher.update(path_blob(repopilotignore_path(path)).as_bytes());
    hasher.update(b"\nignore-sources:");
    hasher.update(ignore_sources.as_bytes());
    hasher.update(b"\ntree:");
    hasher.update(tree.as_bytes());
    Some(hex(&hasher.finalize()))
}

pub(super) fn load(path: &Path, key: &str) -> Option<String> {
    storage::load(path, key)
}

/// Best-effort write: a cache failure must never break the scan.
pub(super) fn store(path: &Path, key: &str, output: &str) {
    storage::store(path, key, output);
}

/// Config content folded into the key so a config change invalidates the cache:
/// the explicit `config` argument, or the same path-discovered default config the
/// MCP scan call uses when `config` is omitted.
fn config_blob(path: &Path, arguments: &Value) -> String {
    let config_path = arguments
        .get("config")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .or_else(|| discover_config_path(config_search_start(path)));
    path_blob(config_path)
}

fn changed_scope_base_blob(repo_root: &Path, arguments: &Value) -> Option<String> {
    if arguments.get("scope").and_then(Value::as_str) != Some("changed") {
        return Some("scope:full-or-default\n".to_string());
    }

    let Some(base) = arguments.get("base").and_then(Value::as_str) else {
        return Some("base:<working-tree>\n".to_string());
    };
    let resolved = git::resolve_commit(repo_root, base)?;
    Some(format!("base:{base}\nresolved:{resolved}\n"))
}

pub(super) fn config_search_start(path: &Path) -> &Path {
    if path.is_file() {
        path.parent().unwrap_or(path)
    } else {
        path
    }
}

fn repopilotignore_path(path: &Path) -> Option<PathBuf> {
    let candidate = config_search_start(path).join(REPOPILOT_IGNORE_FILENAME);
    candidate.is_file().then_some(candidate)
}

fn ignore_sources_blob(path: &Path, repo_root: &Path) -> Option<String> {
    let mut paths = BTreeSet::new();
    for parent in scan_ancestors(path, repo_root)? {
        paths.insert(parent.join(".gitignore"));
        paths.insert(parent.join(".ignore"));
    }
    if let Some(exclude) = git::git_path(repo_root, "info/exclude") {
        paths.insert(exclude);
    }
    for global in global_ignore_paths(repo_root) {
        paths.insert(global);
    }

    let mut blob = String::new();
    for path in paths {
        blob.push_str(&path_blob(Some(path)));
        blob.push('\n');
    }
    Some(blob)
}

fn scan_ancestors(path: &Path, repo_root: &Path) -> Option<Vec<PathBuf>> {
    let start = absolute_scan_start(path);
    let start = start.canonicalize().unwrap_or(start);
    let start = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start
    };
    let mut ancestors = Vec::new();
    let mut current = Some(start.as_path());
    while let Some(dir) = current {
        if !dir.starts_with(repo_root) {
            return None;
        }
        ancestors.push(dir.to_path_buf());
        if dir == repo_root {
            break;
        }
        current = dir.parent();
    }
    Some(ancestors)
}

fn absolute_scan_start(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

fn global_ignore_paths(repo_root: &Path) -> Vec<PathBuf> {
    if let Some(configured) = git::git(
        repo_root,
        &["config", "--path", "--get", "core.excludesFile"],
    ) {
        let path = PathBuf::from(configured);
        return vec![if path.is_absolute() {
            path
        } else {
            repo_root.join(path)
        }];
    }

    fallback_global_ignore_path().into_iter().collect()
}

fn fallback_global_ignore_path() -> Option<PathBuf> {
    if let Some(xdg) = env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        return Some(PathBuf::from(xdg).join("git/ignore"));
    }
    env::var_os("HOME").map(|home| PathBuf::from(home).join(".config/git/ignore"))
}

fn path_blob(path: impl Into<Option<PathBuf>>) -> String {
    let Some(path) = path.into() else {
        return "missing\n".to_string();
    };
    match fs::read(&path) {
        Ok(bytes) => {
            let mut blob = format!("path:{}\nbytes:", path.display());
            blob.push_str(&hex(&bytes));
            blob
        }
        Err(_) => format!("path:{}\nmissing\n", path.display()),
    }
}

/// Canonical key-sorted encoding so equal arguments hash equally regardless of
/// client key order. This is only stable key material, not a JSON renderer.
fn canonical_json(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            let inner: Vec<String> = entries
                .into_iter()
                .map(|(key, value)| format!("{key:?}:{}", canonical_json(value)))
                .collect();
            format!("{{{}}}", inner.join(","))
        }
        Value::Array(items) => {
            let inner: Vec<String> = items.iter().map(canonical_json).collect();
            format!("[{}]", inner.join(","))
        }
        other => other.to_string(),
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests;
