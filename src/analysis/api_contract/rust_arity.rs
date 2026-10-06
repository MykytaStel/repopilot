use super::javascript::{importers_by_target, resolves_to_exporter};
use super::{ApiContractChange, ApiContractOccurrence, JavaScriptContractFactProvider};
use crate::analysis::symbols::{
    ImportedSymbolFact, JavaScriptSymbolFacts, RustCallFact, SymbolKind,
};
use crate::graph::resolver::normalize_path;
use crate::scan::types::CouplingGraph;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};

pub(super) fn detect<P: JavaScriptContractFactProvider>(
    repo_root: &Path,
    modified_exporters: &[PathBuf],
    graph: &CouplingGraph,
    current_files: &HashSet<PathBuf>,
    provider: &mut P,
) -> Vec<ApiContractOccurrence> {
    let importers = importers_by_target(graph, repo_root);
    let mut occurrences = Vec::new();
    for exporter in modified_exporters.iter().filter(|path| is_rust(path)) {
        let exporter = repository_relative(exporter, repo_root);
        occurrences.extend(detect_exporter(
            &exporter,
            repo_root,
            current_files,
            importers.get(&exporter),
            provider,
        ));
    }
    occurrences.sort();
    occurrences.dedup();
    occurrences
}

fn detect_exporter<P: JavaScriptContractFactProvider>(
    exporter: &Path,
    repo_root: &Path,
    current_files: &HashSet<PathBuf>,
    importers: Option<&BTreeSet<PathBuf>>,
    provider: &mut P,
) -> Vec<ApiContractOccurrence> {
    let (Some(before), Some(current), Some(importers)) = (
        provider.pre_change_facts(exporter),
        provider.current_facts(exporter),
        importers,
    ) else {
        return Vec::new();
    };
    let changes = changed_arities(&before, &current);
    if changes.is_empty() {
        return Vec::new();
    }
    let mut occurrences = Vec::new();
    for importer in importers.iter().filter(|path| is_rust(path)) {
        let Some(caller) = provider.current_facts(importer) else {
            continue;
        };
        occurrences.extend(calls_for_importer(
            &caller,
            importer,
            exporter,
            repo_root,
            current_files,
            &changes,
        ));
    }
    occurrences
}

fn changed_arities(
    before: &JavaScriptSymbolFacts,
    current: &JavaScriptSymbolFacts,
) -> BTreeMap<String, (usize, usize)> {
    let previous = unique_arities(before);
    let next = unique_arities(current);
    previous
        .into_iter()
        .filter_map(|(name, before)| {
            let after = *next.get(&name)?;
            (before != after).then_some((name, (before, after)))
        })
        .collect()
}

fn unique_arities(facts: &JavaScriptSymbolFacts) -> BTreeMap<String, usize> {
    let Some(contracts) = facts.rust_contracts.as_ref() else {
        return BTreeMap::new();
    };
    let mut arities = BTreeMap::new();
    let mut duplicates = BTreeSet::new();
    for function in &contracts.functions {
        if duplicates.contains(&function.name) {
            continue;
        }
        if arities
            .insert(function.name.clone(), function.parameter_count)
            .is_some()
        {
            arities.remove(&function.name);
            duplicates.insert(function.name.clone());
        }
    }
    arities
}

fn calls_for_importer(
    caller: &JavaScriptSymbolFacts,
    importer: &Path,
    exporter: &Path,
    repo_root: &Path,
    current_files: &HashSet<PathBuf>,
    changes: &BTreeMap<String, (usize, usize)>,
) -> Vec<ApiContractOccurrence> {
    let Some(calls) = caller.rust_contracts.as_ref().map(|facts| &facts.calls) else {
        return Vec::new();
    };
    let mut occurrences = Vec::new();
    for call in calls {
        let Some((before, after)) = changes.get(&call.imported_name) else {
            continue;
        };
        if call.argument_count != *before || call.argument_count == *after {
            continue;
        }
        if resolves_to_exporter(
            &import_fact(call),
            importer,
            exporter,
            repo_root,
            current_files,
        ) {
            occurrences.push(occurrence(exporter, importer, call, *before, *after));
        }
    }
    occurrences
}

fn import_fact(call: &RustCallFact) -> ImportedSymbolFact {
    ImportedSymbolFact {
        imported_name: call.imported_name.clone(),
        local_name: call.imported_name.clone(),
        kind: SymbolKind::Value,
        module_specifier: call.module_specifier.clone(),
        line_start: call.line_start,
        line_end: call.line_end,
        byte_start: call.byte_start,
        byte_end: call.byte_end,
    }
}

fn occurrence(
    exporter: &Path,
    importer: &Path,
    call: &RustCallFact,
    before: usize,
    after: usize,
) -> ApiContractOccurrence {
    ApiContractOccurrence {
        exporter_path: exporter.to_path_buf(),
        importer_path: importer.to_path_buf(),
        exported_name: call.imported_name.clone(),
        local_name: call.imported_name.clone(),
        symbol_kind: SymbolKind::Value,
        module_specifier: call.module_specifier.clone(),
        line_start: call.line_start,
        line_end: call.line_end,
        byte_start: call.byte_start,
        byte_end: call.byte_end,
        change: ApiContractChange::RustFunctionArity {
            before,
            after,
            call_arguments: call.argument_count,
        },
    }
}

fn is_rust(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "rs")
}

fn repository_relative(path: &Path, repo_root: &Path) -> PathBuf {
    let root = normalize_path(repo_root);
    let path = normalize_path(path);
    if path.is_absolute() {
        path.strip_prefix(root).unwrap_or(&path).to_path_buf()
    } else {
        path
    }
}
