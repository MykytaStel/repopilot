use crate::review::diff::{ChangeStatus, ChangedFile};
use crate::scan::facts::{FileFacts, ScanFacts};
use crate::scan::path_classification::is_low_signal_audit_path;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

pub(super) fn absolutize_scan_fact_paths(facts: &mut ScanFacts, repo_root: &Path) {
    for file in &mut facts.files {
        file.path = absolute_graph_path(repo_root, &file.path);
    }
    facts
        .files
        .sort_by(|left, right| left.path.cmp(&right.path));
    absolutize_path_map(&mut facts.parsed_content_hashes, repo_root);
    absolutize_path_map(&mut facts.import_spans_by_file, repo_root);
    absolutize_path_map(&mut facts.guarded_optional_imports_by_file, repo_root);
}

pub(super) fn apply_changed_context_facts(
    context: &mut ScanFacts,
    repo_root: &Path,
    changed_files: &[ChangedFile],
    patch_files: &[FileFacts],
    patch_import_spans: &BTreeMap<PathBuf, BTreeMap<String, (usize, usize)>>,
    patch_content_hashes: &BTreeMap<PathBuf, String>,
    patch_guarded_optional_imports: &BTreeMap<PathBuf, Vec<String>>,
) {
    let changed_paths = changed_paths(repo_root, changed_files);
    let removed_paths = removed_paths(repo_root, changed_files);
    let patched_paths = patch_file_paths(repo_root, patch_files);
    replace_changed_context_files(
        context,
        repo_root,
        patch_files,
        &removed_paths,
        &patched_paths,
    );
    replace_changed_context_metadata(
        context,
        repo_root,
        &changed_paths,
        patch_files,
        patch_import_spans,
        patch_content_hashes,
        patch_guarded_optional_imports,
    );
}

/// A modified file the cached graph cannot account for. A file the scan
/// policy skips by path (tests, fixtures, examples) is never a graph node, cold
/// or cached, so it needs no patch: without this, changing only tests rebuilt
/// the whole repository context on every review.
pub(super) fn has_unpatched_modified_file(
    repo_root: &Path,
    changed_files: &[ChangedFile],
    patch_files: &[FileFacts],
    include_low_signal: bool,
) -> bool {
    let patched_paths = patch_file_paths(repo_root, patch_files);
    changed_files.iter().any(|file| {
        file.status == ChangeStatus::Modified
            && (include_low_signal || !is_low_signal_audit_path(&file.path))
            && !patched_paths.contains(&repository_relative_path(repo_root, &file.path))
    })
}

fn changed_paths(repo_root: &Path, changed_files: &[ChangedFile]) -> HashSet<PathBuf> {
    changed_files
        .iter()
        .map(|file| repository_relative_path(repo_root, &file.path))
        .collect()
}

fn removed_paths(repo_root: &Path, changed_files: &[ChangedFile]) -> HashSet<PathBuf> {
    changed_files
        .iter()
        .filter(|file| file.status == ChangeStatus::Deleted)
        .map(|file| repository_relative_path(repo_root, &file.path))
        .collect()
}

fn patch_file_paths(repo_root: &Path, patch_files: &[FileFacts]) -> HashSet<PathBuf> {
    patch_files
        .iter()
        .map(|file| repository_relative_path(repo_root, &file.path))
        .collect()
}

fn replace_changed_context_files(
    context: &mut ScanFacts,
    repo_root: &Path,
    patch_files: &[FileFacts],
    removed_paths: &HashSet<PathBuf>,
    patched_paths: &HashSet<PathBuf>,
) {
    context.files.retain(|file| {
        let path = repository_relative_path(repo_root, &file.path);
        !removed_paths.contains(&path) && !patched_paths.contains(&path)
    });
    context.files.extend(patch_files.iter().cloned());
    context
        .files
        .sort_by(|left, right| left.path.cmp(&right.path));
}

fn replace_changed_context_metadata(
    context: &mut ScanFacts,
    repo_root: &Path,
    changed_paths: &HashSet<PathBuf>,
    patch_files: &[FileFacts],
    patch_import_spans: &BTreeMap<PathBuf, BTreeMap<String, (usize, usize)>>,
    patch_content_hashes: &BTreeMap<PathBuf, String>,
    patch_guarded_optional_imports: &BTreeMap<PathBuf, Vec<String>>,
) {
    retain_unchanged_path_entries(&mut context.import_spans_by_file, repo_root, changed_paths);
    retain_unchanged_path_entries(&mut context.parsed_content_hashes, repo_root, changed_paths);
    retain_unchanged_path_entries(
        &mut context.guarded_optional_imports_by_file,
        repo_root,
        changed_paths,
    );
    add_changed_path_metadata(
        context,
        repo_root,
        patch_files,
        patch_import_spans,
        patch_content_hashes,
        patch_guarded_optional_imports,
    );
}

fn add_changed_path_metadata(
    context: &mut ScanFacts,
    repo_root: &Path,
    patch_files: &[FileFacts],
    patch_import_spans: &BTreeMap<PathBuf, BTreeMap<String, (usize, usize)>>,
    patch_content_hashes: &BTreeMap<PathBuf, String>,
    patch_guarded_optional_imports: &BTreeMap<PathBuf, Vec<String>>,
) {
    for file in patch_files {
        let path = absolute_graph_path(repo_root, &file.path);
        if let Some(spans) = patch_import_spans.get(&file.path) {
            context
                .import_spans_by_file
                .insert(path.clone(), spans.clone());
        }
        if let Some(hash) = patch_content_hashes.get(&file.path) {
            context
                .parsed_content_hashes
                .insert(path.clone(), hash.clone());
        }
        if let Some(imports) = patch_guarded_optional_imports.get(&file.path) {
            context
                .guarded_optional_imports_by_file
                .insert(path, imports.clone());
        }
    }
}

fn retain_unchanged_path_entries<T>(
    entries: &mut BTreeMap<PathBuf, T>,
    repo_root: &Path,
    changed_paths: &HashSet<PathBuf>,
) {
    entries.retain(|path, _| !changed_paths.contains(&repository_relative_path(repo_root, path)));
}

fn repository_relative_path(repo_root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(repo_root).unwrap_or(path).to_path_buf()
}

fn absolutize_path_map<T>(paths: &mut BTreeMap<PathBuf, T>, repo_root: &Path) {
    *paths = std::mem::take(paths)
        .into_iter()
        .map(|(path, value)| (absolute_graph_path(repo_root, &path), value))
        .collect();
}

fn absolute_graph_path(repo_root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo_root.join(path)
    }
}
