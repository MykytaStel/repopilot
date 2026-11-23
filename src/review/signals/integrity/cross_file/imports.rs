use super::super::helpers::Body;
use super::super::scan::Scan;
use super::ChangedHelper;
use super::adapters;
use crate::graph::{
    resolve_import,
    resolver::{normalize_path, resolve_unique_python_module},
};
use crate::review::signals::content::ReviewSource;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};

pub(super) fn tables_for(
    path: &Path,
) -> Option<&'static crate::review::signals::tables::IntegrityTables> {
    let extension = path.extension()?.to_str()?;
    crate::languages::integrity_for_extension(extension)
}

pub(super) fn helper_index(
    repo_root: &Path,
    helpers: &[ChangedHelper],
) -> BTreeMap<(PathBuf, String), usize> {
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

pub(super) fn imported_helpers(
    repo_root: &Path,
    importer: &Path,
    source: &ReviewSource,
    current_files: &HashSet<PathBuf>,
    helper_index: &BTreeMap<(PathBuf, String), usize>,
    helpers: &[ChangedHelper],
) -> BTreeMap<usize, Vec<String>> {
    let Some(import_facts) = adapters::imported_helpers(source) else {
        return BTreeMap::new();
    };
    let importer_absolute = normalize_path(&repo_root.join(importer));
    let mut imports = BTreeMap::<usize, Vec<String>>::new();
    let resolve = if source.language_label() == Some("Python") {
        resolve_unique_python_module
    } else {
        resolve_import
    };
    for import in import_facts {
        let Some(resolved) = resolve(
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
    for (index, helper) in helpers.iter().enumerate() {
        if adapters::go_same_package_helpers(
            source,
            importer,
            &helper.path,
            helper.package_name.as_deref(),
            &helper.name,
        ) {
            imports.entry(index).or_default().push(helper.name.clone());
        }
    }
    for local_names in imports.values_mut() {
        local_names.sort();
        local_names.dedup();
    }
    imports
}

pub(super) fn attach_counts(
    scan: &mut Scan,
    source: &ReviewSource,
    imports: &BTreeMap<usize, Vec<String>>,
    helpers: &[ChangedHelper],
    before: bool,
) -> BTreeMap<usize, String> {
    let names = unambiguous_bindings(imports);
    let mut labels = BTreeMap::new();
    for test in &mut scan.tests {
        let (line, start) = test.location();
        let shadowed = if source.language_label() == Some("Python") {
            adapters::shadowed_in_test(source, start)
        } else {
            BTreeSet::new()
        };
        if shadowed.contains("*") {
            continue;
        }
        let mut called_helpers = BTreeSet::new();
        let bodies = names
            .iter()
            .filter_map(|(alias, index)| {
                let index = (*index)?;
                if shadowed.contains(alias.as_str()) || !test.calls_directly(alias) {
                    return None;
                }
                called_helpers.insert(index);
                let helper = &helpers[index];
                let body = if before {
                    &helper.before
                } else {
                    &helper.after
                };
                Some((alias.clone(), body.clone()))
            })
            .collect::<BTreeMap<String, Body>>();
        test.include_external_helpers(&bodies);
        if !before && !called_helpers.is_empty() {
            labels.insert(line, helper_label(&called_helpers, helpers));
        }
    }
    labels
}

fn unambiguous_bindings(imports: &BTreeMap<usize, Vec<String>>) -> BTreeMap<String, Option<usize>> {
    let mut names = BTreeMap::new();
    for (index, aliases) in imports {
        for alias in aliases {
            names
                .entry(alias.clone())
                .and_modify(|binding| *binding = None)
                .or_insert(Some(*index));
        }
    }
    names
}

fn helper_label(called: &BTreeSet<usize>, helpers: &[ChangedHelper]) -> String {
    let mut labels = called
        .iter()
        .map(|&index| {
            let helper = &helpers[index];
            format!(
                "{}::{} (line {})",
                helper.path.display(),
                helper.name,
                helper.line
            )
        })
        .collect::<Vec<_>>();
    labels.sort();
    labels.join(", ")
}
