use super::{JavaScriptContractFactProvider, RemovedExportOccurrence};
use crate::analysis::symbols::{ExportedSymbolFact, ImportedSymbolFact, SymbolKind};
use crate::graph::{resolve_import, resolver::normalize_path};
use crate::scan::types::CouplingGraph;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};

pub(super) fn detect_removed_export_imports<P: JavaScriptContractFactProvider>(
    repo_root: &Path,
    modified_exporters: &[PathBuf],
    graph: &CouplingGraph,
    current_files: &HashSet<PathBuf>,
    provider: &mut P,
) -> Vec<RemovedExportOccurrence> {
    let importers = importers_by_target(graph, repo_root);
    let mut occurrences = Vec::new();

    for exporter in modified_exporters
        .iter()
        .filter(|path| is_supported_path(path))
    {
        let exporter = repository_relative(exporter, repo_root);
        let Some(before) = provider.pre_change_facts(&exporter) else {
            continue;
        };
        let Some(current) = provider.current_facts(&exporter) else {
            continue;
        };
        if current.wildcard_re_export {
            continue;
        }
        let removed = removed_symbols(&before.exports, &current);
        let Some(candidates) = importers.get(&exporter) else {
            continue;
        };
        for importer in candidates.iter().filter(|path| is_supported_path(path)) {
            let Some(facts) = provider.current_facts(importer) else {
                continue;
            };
            occurrences.extend(facts.imports.iter().filter_map(|import| {
                confirmed_occurrence(
                    import,
                    importer,
                    &exporter,
                    &removed,
                    repo_root,
                    current_files,
                )
            }));
        }
    }

    occurrences.sort();
    occurrences.dedup();
    occurrences
}

fn confirmed_occurrence(
    import: &ImportedSymbolFact,
    importer: &Path,
    exporter: &Path,
    removed: &RemovedExports,
    repo_root: &Path,
    current_files: &HashSet<PathBuf>,
) -> Option<RemovedExportOccurrence> {
    if !import.module_specifier.starts_with('.')
        || !removed.matches(&import.imported_name, import.kind)
    {
        return None;
    }
    let importer_absolute = absolute_path(repo_root, importer);
    let exporter_absolute = absolute_path(repo_root, exporter);
    let resolved = resolve_import(
        &import.module_specifier,
        &importer_absolute,
        repo_root,
        current_files,
    );
    if resolved.as_deref() != Some(exporter_absolute.as_path()) {
        return None;
    }

    Some(RemovedExportOccurrence {
        exporter_path: exporter.to_path_buf(),
        importer_path: importer.to_path_buf(),
        exported_name: import.imported_name.clone(),
        local_name: import.local_name.clone(),
        symbol_kind: import.kind,
        module_specifier: import.module_specifier.clone(),
        line_start: import.line_start,
        line_end: import.line_end,
        byte_start: import.byte_start,
        byte_end: import.byte_end,
    })
}

#[derive(Debug, Default)]
struct RemovedExports {
    pairs: BTreeSet<(String, SymbolKind)>,
    vanished: BTreeSet<String>,
}

impl RemovedExports {
    fn matches(&self, name: &str, kind: SymbolKind) -> bool {
        self.vanished.contains(name) || self.pairs.contains(&(name.to_string(), kind))
    }
}

fn removed_symbols(
    before: &[ExportedSymbolFact],
    current: &crate::analysis::symbols::JavaScriptSymbolFacts,
) -> RemovedExports {
    let current_pairs = current
        .exports
        .iter()
        .map(|fact| (fact.name.clone(), fact.kind))
        .collect::<BTreeSet<_>>();
    let current_names = current
        .exports
        .iter()
        .map(|fact| fact.name.as_str())
        .collect::<HashSet<_>>();
    let forwarded = current
        .re_exports
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let pairs = before
        .iter()
        .map(|fact| (fact.name.clone(), fact.kind))
        .filter(|(name, _)| !forwarded.contains(name.as_str()))
        .filter(|fact| !current_pairs.contains(fact))
        .collect::<BTreeSet<_>>();
    let vanished = pairs
        .iter()
        .map(|(name, _)| name.clone())
        .filter(|name| !current_names.contains(name.as_str()))
        .collect();
    RemovedExports { pairs, vanished }
}

fn importers_by_target(
    graph: &CouplingGraph,
    repo_root: &Path,
) -> BTreeMap<PathBuf, BTreeSet<PathBuf>> {
    let mut importers = BTreeMap::<PathBuf, BTreeSet<PathBuf>>::new();
    for (importer, targets) in &graph.edges {
        let importer = repository_relative(importer, repo_root);
        for target in targets {
            importers
                .entry(repository_relative(target, repo_root))
                .or_default()
                .insert(importer.clone());
        }
    }
    importers
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

fn absolute_path(repo_root: &Path, path: &Path) -> PathBuf {
    normalize_path(&if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo_root.join(path)
    })
}

fn is_supported_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("ts" | "tsx" | "js" | "jsx")
    )
}
