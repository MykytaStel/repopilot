mod detector;
mod javascript;

use crate::review::diff::ChangedFile;
use crate::review::diff::DiffTarget;
use crate::scan::types::CouplingGraph;
use std::path::PathBuf;

use crate::review::signals::content::ReviewSource;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SymbolKind {
    Value,
    Type,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ExportedSymbolFact {
    pub name: String,
    pub kind: SymbolKind,
    pub line_start: usize,
    pub line_end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ImportedSymbolFact {
    pub imported_name: String,
    pub local_name: String,
    pub kind: SymbolKind,
    pub module_specifier: String,
    pub line_start: usize,
    pub line_end: usize,
    pub byte_start: usize,
    pub byte_end: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct JavaScriptSymbolFacts {
    pub exports: Vec<ExportedSymbolFact>,
    pub imports: Vec<ImportedSymbolFact>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RemovedExportSignal {
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

pub(crate) struct ChangedReviewSources<'a> {
    pub file: &'a ChangedFile,
    pub pre: Option<&'a ReviewSource>,
    pub post: Option<&'a ReviewSource>,
}

pub(crate) fn detect_removed_export_imports(
    repo_root: &std::path::Path,
    target: DiffTarget<'_>,
    changed_sources: &[ChangedReviewSources<'_>],
    graph: Option<&CouplingGraph>,
) -> Vec<RemovedExportSignal> {
    detector::detect_removed_export_imports(repo_root, target, changed_sources, graph)
}

pub(crate) fn extract_javascript_symbol_facts(
    source: &ReviewSource,
) -> Option<JavaScriptSymbolFacts> {
    javascript::extract_javascript_symbol_facts(source)
}

#[cfg(test)]
mod tests;
