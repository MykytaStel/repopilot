pub(crate) mod javascript;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SymbolKind {
    Value,
    Type,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ExportedSymbolFact {
    pub name: String,
    pub kind: SymbolKind,
    pub line_start: usize,
    pub line_end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct JavaScriptSymbolFacts {
    pub exports: Vec<ExportedSymbolFact>,
    /// Names forwarded by a sourced `export ... from "..."`. Their defining
    /// module is not analyzed, so the symbol kind stays unknown.
    pub re_exports: Vec<String>,
    /// `export * from "..."` is present, so any name may still be supplied.
    pub wildcard_re_export: bool,
    pub imports: Vec<ImportedSymbolFact>,
}
