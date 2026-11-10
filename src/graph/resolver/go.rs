//! Go import resolution from the main module, `go.work` members, and local
//! `replace` directives. Remote module replacements remain outside the graph.

use super::{normalize_path, probe};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

mod workspace;
pub(crate) use workspace::GoIndex;
use workspace::local_modules;

pub(super) fn resolve_go(
    raw: &str,
    from_file: &Path,
    root: &Path,
    known_files: &HashSet<PathBuf>,
) -> Option<PathBuf> {
    let index = GoIndex::new(root);
    resolve_go_with_index(raw, from_file, root, known_files, &index)
}

pub(super) fn resolve_go_with_index(
    raw: &str,
    from_file: &Path,
    root: &Path,
    known_files: &HashSet<PathBuf>,
    index: &GoIndex,
) -> Option<PathBuf> {
    if !raw.contains('/') {
        return None;
    }

    let mut matches = index
        .modules_for_source(from_file)
        .iter()
        .filter_map(|module| {
            let rest = strip_go_module_prefix(raw, &module.import_path)?;
            let base = module.directory.join(rest.trim_start_matches('/'));
            probe_go_package(&normalize_path(&base), known_files)
        })
        .collect::<Vec<_>>();
    matches.sort();
    matches.dedup();
    if matches.len() == 1 {
        return matches.pop();
    }

    // Preserve the historical repository-name fallback for simple fixtures
    // without module metadata.
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
pub(super) fn definitive_local_candidates(
    raw: &str,
    from_file: &Path,
    root: &Path,
) -> Option<Vec<PathBuf>> {
    if raw.contains('\\') || raw.contains('\0') {
        return None;
    }

    let root = normalize_path(root);
    let mut candidates = Vec::new();
    let mut recognized = false;
    for module in local_modules(from_file, &root) {
        let Some(rest) = strip_go_module_prefix(raw, &module.import_path) else {
            continue;
        };
        let relative = rest.trim_start_matches('/');
        let relative_path = Path::new(relative);
        if rest == "/"
            || relative_path
                .components()
                .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            return None;
        }
        let package_dir = normalize_path(&module.directory.join(relative_path));
        if !package_dir.starts_with(&root) {
            return None;
        }
        recognized = true;
        candidates.extend(go_files_in_package(&package_dir));
    }
    if !recognized {
        return None;
    }
    candidates.sort();
    candidates.dedup();
    Some(candidates)
}

fn go_files_in_package(package_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = std::fs::read_dir(package_dir)
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
    candidates
}

pub(super) fn is_local_module_import(raw: &str, source: &Path, root: &Path) -> bool {
    local_modules(source, root)
        .iter()
        .any(|module| strip_go_module_prefix(raw, &module.import_path).is_some())
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
