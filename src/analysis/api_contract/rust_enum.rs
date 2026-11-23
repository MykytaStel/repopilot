use super::rust_arity::{is_rust, repository_relative};
use super::{ApiContractChange, ApiContractOccurrence, JavaScriptContractFactProvider};
use crate::analysis::symbols::{JavaScriptSymbolFacts, RustEnumVariantUseFact, SymbolKind};
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
    let importers = super::javascript::importers_by_target(graph, repo_root);
    let mut occurrences = modified_exporters
        .iter()
        .filter(|path| is_rust(path))
        .flat_map(|exporter| {
            occurrences_for_exporter(exporter, repo_root, &importers, current_files, provider)
        })
        .collect::<Vec<_>>();
    occurrences.sort();
    occurrences.dedup();
    occurrences
}

fn occurrences_for_exporter<P: JavaScriptContractFactProvider>(
    exporter: &Path,
    repo_root: &Path,
    importers_by_target: &std::collections::BTreeMap<PathBuf, std::collections::BTreeSet<PathBuf>>,
    current_files: &HashSet<PathBuf>,
    provider: &mut P,
) -> Vec<ApiContractOccurrence> {
    let exporter = repository_relative(exporter, repo_root);
    let importers = transitive_rust_importers(&exporter, importers_by_target);
    if importers.is_empty() {
        return Vec::new();
    }
    let (Some(before), Some(current)) = (
        provider.pre_change_facts(&exporter),
        provider.current_facts(&exporter),
    ) else {
        return Vec::new();
    };
    let removed = removed_public_variants(&before, &current);
    if removed.is_empty() {
        return Vec::new();
    }
    importers
        .iter()
        .filter_map(|importer| {
            let caller = provider.current_facts(importer)?;
            Some(variant_uses(
                &caller,
                importer,
                &exporter,
                repo_root,
                current_files,
                &removed,
                provider,
            ))
        })
        .flatten()
        .collect()
}

fn transitive_rust_importers(
    exporter: &Path,
    importers_by_target: &std::collections::BTreeMap<PathBuf, std::collections::BTreeSet<PathBuf>>,
) -> BTreeSet<PathBuf> {
    let mut seen = BTreeSet::from([exporter.to_path_buf()]);
    let mut importers = BTreeSet::new();
    let mut pending = vec![exporter.to_path_buf()];
    while let Some(target) = pending.pop() {
        let reachable = importers_by_target
            .get(&target)
            .into_iter()
            .flatten()
            .filter(|importer| is_rust(importer))
            .filter(|importer| seen.insert(importer.to_path_buf()))
            .cloned()
            .collect::<Vec<_>>();
        pending.extend(reachable.iter().cloned());
        importers.extend(reachable);
    }
    importers
}

fn removed_public_variants(
    before: &JavaScriptSymbolFacts,
    current: &JavaScriptSymbolFacts,
) -> BTreeMap<(String, String), (usize, usize)> {
    let (Some(before), Some(current)) = (
        before.rust_contracts.as_ref(),
        current.rust_contracts.as_ref(),
    ) else {
        return BTreeMap::new();
    };
    let old_enums = unique_names(&before.public_enums);
    let new_enums = unique_names(&current.public_enums);
    let old_variants = unique_variants(&before.public_enum_variants);
    let new_variants = unique_variants(&current.public_enum_variants);
    old_variants
        .into_iter()
        .filter(|((enum_name, variant_name), _)| {
            old_enums.contains(enum_name.as_str())
                && new_enums.contains(enum_name.as_str())
                && !new_variants.contains_key(&(enum_name.clone(), variant_name.clone()))
                && !current.inherent_items.iter().any(|(ty, member)| {
                    ty == enum_name.trim_start_matches("r#")
                        && member == variant_name.trim_start_matches("r#")
                })
        })
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

fn unique_variants(
    variants: &[crate::analysis::symbols::RustEnumVariantFact],
) -> BTreeMap<(String, String), (usize, usize)> {
    let mut unique = BTreeMap::new();
    let mut duplicates = BTreeSet::new();
    for variant in variants {
        let key = (variant.enum_name.clone(), variant.variant_name.clone());
        if unique
            .insert(key.clone(), (variant.line_start, variant.line_end))
            .is_some()
        {
            duplicates.insert(key);
        }
    }
    unique.retain(|variant, _| !duplicates.contains(variant));
    unique
}

fn variant_uses<P: JavaScriptContractFactProvider>(
    caller: &JavaScriptSymbolFacts,
    importer: &Path,
    exporter: &Path,
    repo_root: &Path,
    current_files: &HashSet<PathBuf>,
    removed: &BTreeMap<(String, String), (usize, usize)>,
    provider: &mut P,
) -> Vec<ApiContractOccurrence> {
    let mut resolved_modules = BTreeMap::new();
    caller
        .rust_contracts
        .as_ref()
        .map(|facts| facts.enum_variant_uses.as_slice())
        .unwrap_or_default()
        .iter()
        .filter_map(|usage| {
            let key = (usage.enum_name.clone(), usage.variant_name.clone());
            let declaration_lines = removed.get(&key).copied()?;
            let resolved = resolved_modules
                .entry(usage.module_specifier.clone())
                .or_insert_with(|| {
                    rust_module_target(
                        &usage.module_specifier,
                        importer,
                        repo_root,
                        current_files,
                        provider,
                    )
                });
            (resolved.as_deref() == Some(exporter))
                .then(|| occurrence(usage, declaration_lines, importer, exporter))
        })
        .collect()
}

fn rust_module_target<P: JavaScriptContractFactProvider>(
    specifier: &str,
    importer: &Path,
    root: &Path,
    files: &HashSet<PathBuf>,
    provider: &mut P,
) -> Option<PathBuf> {
    use crate::graph::resolver::definitive_local_candidates;

    let modules = if let Some(modules) = specifier.strip_prefix("crate::") {
        if importer != Path::new("src/lib.rs") && importer != Path::new("src/main.rs") {
            return None;
        }
        modules
    } else {
        specifier.strip_prefix("mod::")?
    };
    let mut current = importer.to_path_buf();
    for name in modules
        .split("::")
        .map(|name| name.trim_start_matches("r#"))
    {
        let facts = provider.current_facts(&current)?.rust_contracts?;
        if facts
            .file_modules
            .iter()
            .filter(|module| *module == name)
            .count()
            != 1
        {
            return None;
        }
        let candidates =
            definitive_local_candidates(&format!("mod::{name}"), &root.join(&current), root)?;
        let mut matches = candidates
            .into_iter()
            .filter(|candidate| files.contains(candidate));
        let only = matches.next()?;
        if matches.next().is_some() {
            return None;
        }
        current = only.strip_prefix(root).ok()?.to_path_buf();
    }
    Some(current)
}

fn occurrence(
    usage: &RustEnumVariantUseFact,
    declaration_lines: (usize, usize),
    importer: &Path,
    exporter: &Path,
) -> ApiContractOccurrence {
    ApiContractOccurrence {
        exporter_path: exporter.to_path_buf(),
        importer_path: importer.to_path_buf(),
        exported_name: usage.variant_name.clone(),
        local_name: usage.enum_name.clone(),
        symbol_kind: SymbolKind::Value,
        module_specifier: usage.module_specifier.clone(),
        line_start: usage.line_start,
        line_end: usage.line_end,
        byte_start: usage.byte_start,
        byte_end: usage.byte_end,
        declaration_lines: (declaration_lines.0 > 0).then_some(declaration_lines),
        change: ApiContractChange::RustEnumVariantRemoved,
    }
}
