mod javascript;

use crate::analysis::symbols::{JavaScriptSymbolFacts, SymbolKind};
use crate::scan::types::CouplingGraph;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RemovedExportOccurrence {
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
}

pub(crate) trait JavaScriptContractFactProvider {
    fn pre_change_facts(&mut self, path: &Path) -> Option<JavaScriptSymbolFacts>;
    fn current_facts(&mut self, path: &Path) -> Option<JavaScriptSymbolFacts>;
}

pub(crate) fn detect_removed_export_imports<P: JavaScriptContractFactProvider>(
    repo_root: &Path,
    modified_exporters: &[PathBuf],
    graph: &CouplingGraph,
    current_files: &HashSet<PathBuf>,
    provider: &mut P,
) -> Vec<RemovedExportOccurrence> {
    javascript::detect_removed_export_imports(
        repo_root,
        modified_exporters,
        graph,
        current_files,
        provider,
    )
}

#[cfg(test)]
mod tests;
