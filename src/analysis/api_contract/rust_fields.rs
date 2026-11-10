use super::rust_arity::{is_rust, repository_relative};
use super::{ApiContractChange, ApiContractOccurrence, JavaScriptContractFactProvider};
use crate::analysis::symbols::{
    ImportedSymbolFact, JavaScriptSymbolFacts, RustFieldUseFact, SymbolKind,
};
use crate::scan::types::CouplingGraph;
use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};

pub(super) fn detect<P: JavaScriptContractFactProvider>(
    repo_root: &Path,
    modified_exporters: &[PathBuf],
    graph: &CouplingGraph,
    current_files: &HashSet<PathBuf>,
    provider: &mut P,
) -> Vec<ApiContractOccurrence> {
    let importers = super::javascript::importers_by_target(graph, repo_root);
    let mut occurrences = Vec::new();
    for exporter in modified_exporters.iter().filter(|path| is_rust(path)) {
        let exporter = repository_relative(exporter, repo_root);
        let Some(importers) = importers.get(&exporter) else {
            continue;
        };
        let (Some(before), Some(current)) = (
            provider.pre_change_facts(&exporter),
            provider.current_facts(&exporter),
        ) else {
            continue;
        };
        let removed = removed_public_fields(&before, &current);
        if removed.is_empty() {
            continue;
        }
        for importer in importers.iter().filter(|path| is_rust(path)) {
            let Some(caller) = provider.current_facts(importer) else {
                continue;
            };
            occurrences.extend(field_uses(
                &caller,
                importer,
                &exporter,
                repo_root,
                current_files,
                &removed,
            ));
        }
    }
    occurrences.sort();
    occurrences.dedup();
    occurrences
}

fn removed_public_fields(
    before: &JavaScriptSymbolFacts,
    current: &JavaScriptSymbolFacts,
) -> BTreeSet<(String, String)> {
    let Some(previous) = before.rust_contracts.as_ref() else {
        return BTreeSet::new();
    };
    let Some(next) = current.rust_contracts.as_ref() else {
        return BTreeSet::new();
    };
    let old_structs = unique_names(&previous.public_structs);
    let new_structs = unique_names(&next.public_structs);
    let old_fields = previous
        .public_struct_fields
        .iter()
        .map(|field| (field.struct_name.as_str(), field.field_name.as_str()))
        .collect::<BTreeSet<_>>();
    let new_fields = next
        .public_struct_fields
        .iter()
        .map(|field| (field.struct_name.as_str(), field.field_name.as_str()))
        .collect::<BTreeSet<_>>();
    old_fields
        .difference(&new_fields)
        .filter(|(struct_name, _)| {
            old_structs.contains(*struct_name) && new_structs.contains(*struct_name)
        })
        .map(|(struct_name, field_name)| ((*struct_name).to_string(), (*field_name).to_string()))
        .collect()
}

fn unique_names(names: &[String]) -> BTreeSet<&str> {
    let mut unique = BTreeSet::new();
    let mut duplicates = BTreeSet::new();
    for name in names {
        if !unique.insert(name.as_str()) {
            duplicates.insert(name.as_str());
        }
    }
    unique.retain(|name| !duplicates.contains(name));
    unique
}

fn field_uses(
    caller: &JavaScriptSymbolFacts,
    importer: &Path,
    exporter: &Path,
    repo_root: &Path,
    current_files: &HashSet<PathBuf>,
    removed: &BTreeSet<(String, String)>,
) -> Vec<ApiContractOccurrence> {
    let Some(uses) = caller
        .rust_contracts
        .as_ref()
        .map(|facts| &facts.struct_field_uses)
    else {
        return Vec::new();
    };
    uses.iter()
        .filter(|field| removed.contains(&(field.struct_name.clone(), field.field_name.clone())))
        .filter_map(|field| occurrence(field, importer, exporter, repo_root, current_files))
        .collect()
}

fn occurrence(
    field: &RustFieldUseFact,
    importer: &Path,
    exporter: &Path,
    repo_root: &Path,
    current_files: &HashSet<PathBuf>,
) -> Option<ApiContractOccurrence> {
    let import = ImportedSymbolFact {
        imported_name: field.struct_name.clone(),
        local_name: field.struct_name.clone(),
        kind: SymbolKind::Type,
        module_specifier: field.module_specifier.clone(),
        line_start: field.line_start,
        line_end: field.line_end,
        byte_start: field.byte_start,
        byte_end: field.byte_end,
    };
    super::javascript::resolves_to_exporter(&import, importer, exporter, repo_root, current_files)
        .then(|| ApiContractOccurrence {
            exporter_path: exporter.to_path_buf(),
            importer_path: importer.to_path_buf(),
            exported_name: field.field_name.clone(),
            local_name: field.field_name.clone(),
            symbol_kind: SymbolKind::Value,
            module_specifier: field.module_specifier.clone(),
            line_start: field.line_start,
            line_end: field.line_end,
            byte_start: field.byte_start,
            byte_end: field.byte_end,
            change: ApiContractChange::RustStructFieldRemoved,
        })
}
