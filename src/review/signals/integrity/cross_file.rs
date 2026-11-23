//! One-hop attribution from changed local assertion helpers to unchanged tests.

#[path = "cross_file/adapters.rs"]
mod adapters;
#[path = "cross_file/imports.rs"]
mod imports;

use super::accounting;
use super::helpers::{self, Body};
use super::scan;
use super::{ChangedIntegritySource, FileEvidence, IntegritySignal, is_test_input_data};
use crate::graph::resolver::normalize_path;
use crate::review::diff::{DiffTarget, target_file_inventory};
use crate::review::paths::normalized_review_path;
use crate::review::signals::content::{
    ReviewSource, post_change_source_at_path, pre_change_source_at_path,
};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};

pub(super) struct ChangedHelper {
    path: PathBuf,
    name: String,
    line: usize,
    before: Body,
    after: Body,
    package_name: Option<String>,
}

pub(super) fn detect(
    repo_root: &Path,
    target: DiffTarget<'_>,
    changed: &[ChangedIntegritySource<'_>],
    _evidence: &[FileEvidence],
) -> Vec<IntegritySignal> {
    let changed_paths = changed
        .iter()
        .map(|source| normalized_review_path(&source.file.path, repo_root))
        .collect::<BTreeSet<_>>();
    let helpers = changed_helpers(repo_root, changed);
    if helpers.is_empty() {
        return Vec::new();
    }
    let Some(current_files) = target_file_inventory(repo_root, target) else {
        return Vec::new();
    };
    let current_files = absolute_files(repo_root, current_files);
    let helper_index = imports::helper_index(repo_root, &helpers);
    let mut signals = Vec::new();

    for importer in importers(repo_root, &current_files, &changed_paths) {
        signals.extend(signals_for_importer(
            repo_root,
            target,
            &importer,
            &current_files,
            &helper_index,
            &helpers,
        ));
    }
    signals.sort_by(|left, right| left.path.cmp(&right.path).then(left.line.cmp(&right.line)));
    signals.dedup();
    signals
}

fn signals_for_importer(
    repo_root: &Path,
    target: DiffTarget<'_>,
    importer: &Path,
    current_files: &HashSet<PathBuf>,
    helper_index: &BTreeMap<(PathBuf, String), usize>,
    helpers: &[ChangedHelper],
) -> Vec<IntegritySignal> {
    let Some((before_source, after_source, imports)) = caller_sources(
        repo_root,
        target,
        importer,
        current_files,
        helper_index,
        helpers,
    ) else {
        return Vec::new();
    };
    let Some(tables) = imports::tables_for(importer) else {
        return Vec::new();
    };
    let Some(mut before) = scan::scan(&before_source, tables, true) else {
        return Vec::new();
    };
    let Some(mut after) = scan::scan(&after_source, tables, true) else {
        return Vec::new();
    };
    imports::attach_counts(&mut before, &before_source, &imports, helpers, true);
    let labels = imports::attach_counts(&mut after, &after_source, &imports, helpers, false);
    let path = importer.to_string_lossy().into_owned();
    accounting::cross_file_signals(&path, &before.tests, &after.tests, &labels)
}

fn caller_sources(
    repo_root: &Path,
    target: DiffTarget<'_>,
    importer: &Path,
    current_files: &HashSet<PathBuf>,
    helper_index: &BTreeMap<(PathBuf, String), usize>,
    helpers: &[ChangedHelper],
) -> Option<(ReviewSource, ReviewSource, BTreeMap<usize, Vec<String>>)> {
    let after_source = post_change_source_at_path(repo_root, importer, target)?;
    let after_imports = imports::imported_helpers(
        repo_root,
        importer,
        &after_source,
        current_files,
        helper_index,
        helpers,
    );
    if after_imports.is_empty() {
        return None;
    }
    let before_source = pre_change_source_at_path(repo_root, importer, target)?;
    let before_imports = imports::imported_helpers(
        repo_root,
        importer,
        &before_source,
        current_files,
        helper_index,
        helpers,
    );
    (before_imports == after_imports).then_some((before_source, after_source, after_imports))
}

fn changed_helpers(repo_root: &Path, changed: &[ChangedIntegritySource<'_>]) -> Vec<ChangedHelper> {
    changed
        .iter()
        .flat_map(|source| changed_helpers_in_file(repo_root, source))
        .collect()
}

fn changed_helpers_in_file(
    repo_root: &Path,
    source: &ChangedIntegritySource<'_>,
) -> Vec<ChangedHelper> {
    if source.file.status != crate::review::diff::ChangeStatus::Modified
        || crate::audits::context::classify::helpers::is_test_file(&source.file.path)
    {
        return Vec::new();
    }
    let (Some(before_source), Some(after_source)) = (source.pre, source.post) else {
        return Vec::new();
    };
    let path = normalized_review_path(&source.file.path, repo_root);
    let Some(tables) = imports::tables_for(&path) else {
        return Vec::new();
    };
    let (Some(pre_scan), Some(post_scan)) = (
        scan::scan(before_source, tables, true),
        scan::scan(after_source, tables, true),
    ) else {
        return Vec::new();
    };
    let (Some(before_declarations), Some(mut after_declarations)) = (
        adapters::declared_helpers(before_source),
        adapters::declared_helpers(after_source),
    ) else {
        return Vec::new();
    };
    if before_declarations.package_name != after_declarations.package_name {
        return Vec::new();
    }
    after_declarations
        .lines
        .retain(|name, _| before_declarations.lines.contains_key(name));
    changed_helper_candidates(path, &after_declarations, pre_scan, post_scan)
}

fn changed_helper_candidates(
    path: PathBuf,
    declarations: &adapters::DeclaredHelpers,
    before: scan::Scan,
    after: scan::Scan,
) -> Vec<ChangedHelper> {
    before
        .helpers
        .iter()
        .filter_map(|(name, before_body)| {
            if before.ambiguous_helpers.contains(name) || after.ambiguous_helpers.contains(name) {
                return None;
            }
            let line = *declarations.lines.get(name)?;
            let after_body = after.helpers.get(name)?;
            let before_body = helpers::resolve(before_body, &before.helpers);
            let after_body = helpers::resolve(after_body, &after.helpers);
            helper_checks_differ(&before_body, &after_body).then(|| ChangedHelper {
                path: path.clone(),
                name: name.clone(),
                line,
                before: before_body,
                after: after_body,
                package_name: declarations.package_name.clone(),
            })
        })
        .collect()
}

fn helper_checks_differ(before: &Body, after: &Body) -> bool {
    before.assertions != after.assertions
        || before.exact_matchers != after.exact_matchers
        || before.partial_matchers != after.partial_matchers
        || before.containing_matchers != after.containing_matchers
}

fn importers(
    repo_root: &Path,
    current_files: &HashSet<PathBuf>,
    changed: &BTreeSet<PathBuf>,
) -> BTreeSet<PathBuf> {
    // The repository graph intentionally omits low-signal test paths. Enumerate
    // only test files with integrity tables here; imported_helpers then proves
    // each edge using the named symbol fact and exact path resolver.
    current_files
        .iter()
        .map(|path| normalized_review_path(path, repo_root))
        .filter(|path| {
            !changed.contains(path)
                && crate::audits::context::classify::helpers::is_test_file(path)
                && !is_test_input_data(path)
                && imports::tables_for(path).is_some()
        })
        .collect()
}

fn absolute_files(repo_root: &Path, files: HashSet<PathBuf>) -> HashSet<PathBuf> {
    files
        .into_iter()
        .map(|path| {
            normalize_path(&if path.is_absolute() {
                path
            } else {
                repo_root.join(path)
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "cross_file/tests.rs"]
mod tests;
