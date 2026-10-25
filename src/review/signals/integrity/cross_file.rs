//! One-hop attribution from changed JS/TS assertion helpers to unchanged tests.

use super::accounting;
use super::helpers::{self, Body};
use super::scan::{self, Scan};
use super::{ChangedIntegritySource, FileEvidence, IntegritySignal, is_test_input_data};
use crate::analysis::symbols::SymbolKind;
use crate::graph::{resolve_import, resolver::normalize_path};
use crate::review::diff::{DiffTarget, target_file_inventory};
use crate::review::paths::normalized_review_path;
use crate::review::signals::api_contract::extract_javascript_symbol_facts;
use crate::review::signals::content::{
    ReviewSource, post_change_source_at_path, pre_change_source_at_path,
};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};

struct ChangedHelper {
    path: PathBuf,
    name: String,
    before: Body,
    after: Body,
}

pub(super) fn detect(
    repo_root: &Path,
    target: DiffTarget<'_>,
    changed: &[ChangedIntegritySource<'_>],
    evidence: &[FileEvidence],
) -> Vec<IntegritySignal> {
    let Some(current_files) = target_file_inventory(repo_root, target) else {
        return Vec::new();
    };
    let current_files = absolute_files(repo_root, current_files);
    let evidence_by_path = evidence_by_path(repo_root, evidence);
    let changed_paths = changed
        .iter()
        .map(|source| normalized_review_path(&source.file.path, repo_root))
        .collect::<BTreeSet<_>>();
    let helpers = changed_helpers(repo_root, changed, &evidence_by_path);
    if helpers.is_empty() {
        return Vec::new();
    }
    let helper_index = helper_index(repo_root, &helpers);
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
    let Some((before_source, after_source, imports)) =
        caller_sources(repo_root, target, importer, current_files, helper_index)
    else {
        return Vec::new();
    };
    let Some(tables) = tables_for(importer) else {
        return Vec::new();
    };
    let Some(mut before) = scan::scan(&before_source, tables, true) else {
        return Vec::new();
    };
    let Some(mut after) = scan::scan(&after_source, tables, true) else {
        return Vec::new();
    };
    attach_counts(&mut before, &effective_helpers(&imports, helpers, true));
    attach_counts(&mut after, &effective_helpers(&imports, helpers, false));
    let path = importer.to_string_lossy().into_owned();
    let label = helper_label(&imports, helpers);
    accounting::cross_file_signals(&path, &before.tests, &after.tests, &label)
}

fn caller_sources(
    repo_root: &Path,
    target: DiffTarget<'_>,
    importer: &Path,
    current_files: &HashSet<PathBuf>,
    helper_index: &BTreeMap<(PathBuf, String), usize>,
) -> Option<(ReviewSource, ReviewSource, BTreeMap<usize, Vec<String>>)> {
    let after_source = post_change_source_at_path(repo_root, importer, target)?;
    let after_imports = imported_helpers(
        repo_root,
        importer,
        &after_source,
        current_files,
        helper_index,
    );
    if after_imports.is_empty() {
        return None;
    }
    let before_source = pre_change_source_at_path(repo_root, importer, target)?;
    let before_imports = imported_helpers(
        repo_root,
        importer,
        &before_source,
        current_files,
        helper_index,
    );
    (before_imports == after_imports).then_some((before_source, after_source, after_imports))
}

fn changed_helpers(
    repo_root: &Path,
    changed: &[ChangedIntegritySource<'_>],
    evidence: &BTreeMap<PathBuf, &FileEvidence>,
) -> Vec<ChangedHelper> {
    changed
        .iter()
        .flat_map(|source| changed_helpers_in_file(repo_root, source, evidence))
        .collect()
}

fn changed_helpers_in_file(
    repo_root: &Path,
    source: &ChangedIntegritySource<'_>,
    evidence: &BTreeMap<PathBuf, &FileEvidence>,
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
    let Some(file) = evidence.get(&path).copied() else {
        return Vec::new();
    };
    let (Some(before_names), Some(after_names)) = (
        exported_value_names(before_source),
        exported_value_names(after_source),
    ) else {
        return Vec::new();
    };

    before_names
        .intersection(&after_names)
        .filter_map(|name| {
            if file.pre.ambiguous_helpers.contains(name)
                || file.post.ambiguous_helpers.contains(name)
            {
                return None;
            }
            let (before, after) = (file.pre.helpers.get(name)?, file.post.helpers.get(name)?);
            let before = helpers::resolve(before, &file.pre.helpers);
            let after = helpers::resolve(after, &file.post.helpers);
            helper_checks_differ(&before, &after).then(|| ChangedHelper {
                path: path.clone(),
                name: name.clone(),
                before,
                after,
            })
        })
        .collect()
}

fn exported_value_names(source: &ReviewSource) -> Option<BTreeSet<String>> {
    extract_javascript_symbol_facts(source).map(|facts| {
        facts
            .exports
            .into_iter()
            .filter(|export| export.kind == SymbolKind::Value)
            .map(|export| export.name)
            .collect()
    })
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
                && tables_for(path).is_some()
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

fn evidence_by_path<'a>(
    repo_root: &Path,
    evidence: &'a [FileEvidence],
) -> BTreeMap<PathBuf, &'a FileEvidence> {
    evidence
        .iter()
        .map(|file| {
            (
                normalized_review_path(Path::new(&file.path), repo_root),
                file,
            )
        })
        .collect()
}

fn tables_for(path: &Path) -> Option<&'static crate::review::signals::tables::IntegrityTables> {
    let extension = path.extension()?.to_str()?;
    let tables = crate::languages::integrity_for_extension(extension)?;
    tables.assertion_matcher.map(|_| tables)
}

fn helper_index(repo_root: &Path, helpers: &[ChangedHelper]) -> BTreeMap<(PathBuf, String), usize> {
    helpers
        .iter()
        .enumerate()
        .map(|(index, helper)| {
            (
                (
                    normalize_path(&repo_root.join(&helper.path)),
                    helper.name.clone(),
                ),
                index,
            )
        })
        .collect()
}

fn imported_helpers(
    repo_root: &Path,
    importer: &Path,
    source: &ReviewSource,
    current_files: &HashSet<PathBuf>,
    helper_index: &BTreeMap<(PathBuf, String), usize>,
) -> BTreeMap<usize, Vec<String>> {
    let Some(facts) = extract_javascript_symbol_facts(source) else {
        return BTreeMap::new();
    };
    let importer_absolute = normalize_path(&repo_root.join(importer));
    let mut imports = BTreeMap::<usize, Vec<String>>::new();
    for import in facts.imports {
        if import.kind != SymbolKind::Value {
            continue;
        }
        let Some(resolved) = resolve_import(
            &import.module_specifier,
            &importer_absolute,
            repo_root,
            current_files,
        ) else {
            continue;
        };
        if let Some(index) = helper_index.get(&(normalize_path(&resolved), import.imported_name)) {
            imports.entry(*index).or_default().push(import.local_name);
        }
    }
    for local_names in imports.values_mut() {
        local_names.sort();
        local_names.dedup();
    }
    imports
}

fn effective_helpers(
    imports: &BTreeMap<usize, Vec<String>>,
    helpers: &[ChangedHelper],
    before: bool,
) -> BTreeMap<String, Body> {
    imports
        .iter()
        .flat_map(|(index, local_names)| {
            local_names
                .iter()
                .map(move |local_name| (*index, local_name))
        })
        .fold(
            BTreeMap::<String, Body>::new(),
            |mut effective, (index, local_name)| {
                let helper = &helpers[index];
                let body = if before {
                    &helper.before
                } else {
                    &helper.after
                };
                let merged = effective.entry(local_name.clone()).or_default();
                merged.assertions = merged.assertions.saturating_add(body.assertions);
                merged.exact_matchers = merged.exact_matchers.saturating_add(body.exact_matchers);
                merged.partial_matchers = merged
                    .partial_matchers
                    .saturating_add(body.partial_matchers);
                merged.containing_matchers = merged
                    .containing_matchers
                    .saturating_add(body.containing_matchers);
                merged.looped |= body.looped;
                effective
            },
        )
}

fn helper_label(imports: &BTreeMap<usize, Vec<String>>, helpers: &[ChangedHelper]) -> String {
    let mut labels = imports
        .keys()
        .map(|index| {
            let helper = &helpers[*index];
            format!("{}::{}", helper.path.display(), helper.name)
        })
        .collect::<Vec<_>>();
    labels.sort();
    labels.join(", ")
}

fn attach_counts(scan: &mut Scan, helpers: &BTreeMap<String, Body>) {
    for test in &mut scan.tests {
        test.include_external_helpers(helpers);
    }
}
