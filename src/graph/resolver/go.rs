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

/// Enumerates source files for an import inside the module declared by the
/// repository's `go.mod`.
///
/// Go packages are directories rather than one conventionally named file, so
/// the candidate set is the package's non-test `.go` files. A `replace`
/// directive can redirect even an otherwise local-looking module path outside
/// this repository; those imports remain limited instead of becoming a false
/// missing-package finding.
pub(super) fn definitive_local_candidates(raw: &str, root: &Path) -> Option<Vec<PathBuf>> {
    let module_name = read_go_module_name(root)?;
    let rest = strip_go_module_prefix(raw, &module_name)?;
    if go_mod_has_replace(root) || rest == "/" || rest.contains('\\') || rest.contains('\0') {
        return None;
    }

    let relative = rest.trim_start_matches('/');
    let relative_path = Path::new(relative);
    if relative_path
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return None;
    }
    let package_dir = normalize_path(&root.join(relative_path));
    let root = normalize_path(root);
    if !package_dir.starts_with(&root) {
        return None;
    }

    let mut candidates = std::fs::read_dir(&package_dir)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(Result::ok))
        .filter_map(|entry| {
            let file_type = entry.file_type().ok()?;
            if !file_type.is_file() {
                return None;
            }
            let path = entry.path();
            let name = path.file_name()?.to_str()?;
            (path.extension().and_then(|extension| extension.to_str()) == Some("go")
                && !name.ends_with("_test.go"))
            .then_some(path)
        })
        .collect::<Vec<_>>();
    candidates.sort();
    Some(candidates)
}

pub(super) fn is_local_module_import(raw: &str, root: &Path) -> bool {
    read_go_module_name(root)
        .and_then(|module| strip_go_module_prefix(raw, &module))
        .is_some()
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

fn go_mod_has_replace(root: &Path) -> bool {
    std::fs::read_to_string(root.join("go.mod")).is_ok_and(|content| {
        content.lines().any(|line| {
            let trimmed = line.trim();
            trimmed.split_whitespace().next() == Some("replace")
        })
    })
}
