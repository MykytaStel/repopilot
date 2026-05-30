//! Go import resolution, using the `go.mod` module path (cached per repo root)
//! with a fallback to the repository directory name.

use super::{normalize_path, probe};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

static GO_MODULE_CACHE: OnceLock<RwLock<HashMap<PathBuf, Option<String>>>> = OnceLock::new();

fn get_go_module_cache() -> &'static RwLock<HashMap<PathBuf, Option<String>>> {
    GO_MODULE_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

pub(super) fn resolve_go(
    raw: &str,
    root: &Path,
    known_files: &HashSet<PathBuf>,
) -> Option<PathBuf> {
    if !raw.contains('/') {
        return None;
    }

    if let Some(module_name) = read_go_module_name(root)
        && let Some(rest) = strip_go_module_prefix(raw, &module_name)
    {
        let rel = rest.trim_start_matches('/');
        let base = if rel.is_empty() {
            root.to_path_buf()
        } else {
            root.join(rel)
        };
        if let Some(path) = probe_go_package(&normalize_path(&base), known_files) {
            return Some(path);
        }
    }

    let root_name = root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    if !root_name.is_empty()
        && let Some(rest) = strip_go_module_prefix(raw, root_name)
    {
        let rel = rest.trim_start_matches('/');
        let base = normalize_path(&root.join(rel));
        return probe_go_package(&base, known_files);
    }

    None
}

fn strip_go_module_prefix<'a>(raw: &'a str, module_name: &str) -> Option<&'a str> {
    raw.strip_prefix(module_name)
        .filter(|rest| rest.is_empty() || rest.starts_with('/'))
}

fn probe_go_package(base: &Path, known_files: &HashSet<PathBuf>) -> Option<PathBuf> {
    if let Some(path) = probe(&[base.with_extension("go")], known_files) {
        return Some(path);
    }

    let package_dir = normalize_path(base);
    known_files
        .iter()
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("go"))
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_none_or(|name| !name.ends_with("_test.go"))
        })
        .filter(|path| path.parent() == Some(package_dir.as_path()))
        .min()
        .cloned()
}

fn read_go_module_name(root: &Path) -> Option<String> {
    let cache = get_go_module_cache();
    if let Some(cached) = cache.read().unwrap().get(root) {
        return cached.clone();
    }

    let module = std::fs::read_to_string(root.join("go.mod"))
        .ok()
        .and_then(|content| {
            content.lines().find_map(|line| {
                line.trim()
                    .strip_prefix("module ")
                    .map(|module| module.trim().to_string())
            })
        });
    cache
        .write()
        .unwrap()
        .insert(root.to_path_buf(), module.clone());
    module
}
