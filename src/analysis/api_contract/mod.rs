mod javascript;
mod rust_arity;

use crate::analysis::symbols::{JavaScriptSymbolFacts, SymbolKind};
use crate::scan::types::CouplingGraph;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ApiContractChange {
    RemovedExport,
    RustFunctionArity {
        before: usize,
        after: usize,
        call_arguments: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ApiContractOccurrence {
    pub exporter_path: PathBuf,
    pub importer_path: PathBuf,
    pub exported_name: String,
    pub local_name: String,
    pub symbol_kind: SymbolKind,
    pub module_specifier: String,
    pub line_start: usize,
    pub line_end: usize,
    pub byte_start: usize,
    pub byte_end: usize,
    pub change: ApiContractChange,
}

pub(crate) type RemovedExportOccurrence = ApiContractOccurrence;

pub(crate) trait JavaScriptContractFactProvider {
    fn pre_change_facts(&mut self, path: &Path) -> Option<JavaScriptSymbolFacts>;
    fn current_facts(&mut self, path: &Path) -> Option<JavaScriptSymbolFacts>;
}

pub(crate) fn detect_api_contract_breaks<P: JavaScriptContractFactProvider>(
    repo_root: &Path,
    modified_exporters: &[PathBuf],
    graph: &CouplingGraph,
    current_files: &HashSet<PathBuf>,
    provider: &mut P,
) -> Vec<ApiContractOccurrence> {
    let mut occurrences = javascript::detect_removed_export_imports(
        repo_root,
        modified_exporters,
        graph,
        current_files,
        provider,
    );
    occurrences.extend(rust_arity::detect(
        repo_root,
        modified_exporters,
        graph,
        current_files,
        provider,
    ));
    occurrences.sort();
    occurrences.dedup();
    occurrences
}

pub(crate) fn detect_removed_export_imports<P: JavaScriptContractFactProvider>(
    repo_root: &Path,
    modified_exporters: &[PathBuf],
    graph: &CouplingGraph,
    current_files: &HashSet<PathBuf>,
    provider: &mut P,
) -> Vec<ApiContractOccurrence> {
    detect_api_contract_breaks(
        repo_root,
        modified_exporters,
        graph,
        current_files,
        provider,
    )
}

#[cfg(test)]
mod tests;
