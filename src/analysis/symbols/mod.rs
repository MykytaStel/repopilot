pub(crate) mod javascript;
mod rust;

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

/// Shared private direct-symbol facts; the historical name/cache field is
/// retained for JS/TS callers. Rust supplies only public free-function values.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct JavaScriptSymbolFacts {
    pub exports: Vec<ExportedSymbolFact>,
    /// Names forwarded by a sourced `export ... from "..."`. Their defining
    /// module is not analyzed, so the symbol kind stays unknown.
    pub re_exports: Vec<String>,
    /// `export * from "..."` is present, so any name may still be supplied.
    pub wildcard_re_export: bool,
    pub imports: Vec<ImportedSymbolFact>,
    /// Rust-only facts for bounded public-function arity checks.
    #[serde(default)]
    pub rust_contracts: Option<RustContractFacts>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct RustContractFacts {
    pub functions: Vec<RustFunctionArityFact>,
    pub calls: Vec<RustCallFact>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RustFunctionArityFact {
    pub name: String,
    pub parameter_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RustCallFact {
    pub imported_name: String,
    pub module_specifier: String,
    pub line_start: usize,
    pub line_end: usize,
    pub byte_start: usize,
    pub byte_end: usize,
    pub argument_count: usize,
}

/// Dispatch once for all analysis adapters while retaining the private cache
/// representation used by existing JS/TS artifacts.
pub(crate) fn extract_symbol_facts(
    content: &str,
    language: Option<&str>,
    tree: &tree_sitter::Tree,
) -> Option<JavaScriptSymbolFacts> {
    if language == Some("Rust") {
        rust::extract(content, tree)
    } else {
        javascript::extract_javascript_symbol_facts(content, language, tree)
    }
}
