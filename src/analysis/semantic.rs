//! Experimental module-surface index for the semantic analysis pilot.
//!
//! This is intentionally not a compiler symbol table: an observed export is
//! not necessarily the declaration that owns the binding. Resolution is
//! conditional on the caller-supplied source inventory. No absence finding
//! or blocking decision may be inferred from this index alone.

use super::parse::ParsedFile;
use super::symbols::{ImportedSymbolFact, SymbolKind, extract_symbol_facts};
use crate::graph::resolver::{definitive_local_candidates, normalize_path};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

/// Bounded per-module source budget for the opt-in pilot.
pub const MAX_MODULE_BYTES: usize = 1024 * 1024;

/// Source already acquired by the caller. Paths must be repository-relative.
pub struct SourceModule<'a> {
    pub path: &'a Path,
    pub content: &'a str,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum IndexError {
    #[error("symbol index root must be absolute and normalized")]
    InvalidRoot,
    #[error("invalid repository-relative module path: {0:?}")]
    InvalidPath(PathBuf),
    #[error("duplicate module path: {0:?}")]
    DuplicatePath(PathBuf),
}

/// Syntactic namespace, not TypeScript's full type/value namespace model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Namespace {
    Value,
    Type,
}

impl From<SymbolKind> for Namespace {
    fn from(kind: SymbolKind) -> Self {
        match kind {
            SymbolKind::Value => Self::Value,
            SymbolKind::Type => Self::Type,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModuleCoverage {
    IndexedSurface,
    UnsupportedLanguage,
    SourceBudgetExceeded,
    ParseUnavailable,
    SyntaxErrors,
}

/// Snapshot-local identity of a syntactically observed export, not a
/// cross-revision identifier or proof of the underlying declaration.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct ExportSymbol {
    pub module: PathBuf,
    pub name: String,
    pub namespace: Namespace,
    pub line_start: usize,
    pub line_end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModuleSurface {
    pub coverage: ModuleCoverage,
    pub exports: BTreeMap<String, Vec<ExportSymbol>>,
    pub forwarded_names: BTreeSet<String>,
    pub wildcard_forwarding: bool,
}

impl ModuleSurface {
    fn unavailable(coverage: ModuleCoverage) -> Self {
        Self {
            coverage,
            exports: BTreeMap::new(),
            forwarded_names: BTreeSet::new(),
            wildcard_forwarding: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnresolvedReason {
    UnsupportedSpecifier,
    ModuleNotObserved,
    ModuleFactsUnavailable,
    ForwardedExport,
    ExportNotObserved,
    NamespaceUncertain,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum Resolution {
    UniqueObserved { export: ExportSymbol },
    AmbiguousModules { candidates: Vec<PathBuf> },
    AmbiguousExports { candidates: Vec<ExportSymbol> },
    Unresolved { reason: UnresolvedReason },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportReference {
    pub module: PathBuf,
    pub module_specifier: String,
    pub imported_name: String,
    pub local_name: String,
    pub namespace: Namespace,
    pub line_start: usize,
    pub line_end: usize,
    pub byte_start: usize,
    pub byte_end: usize,
    pub resolution: Resolution,
}

/// Deterministic, in-memory, opt-in output. It does not alter scan findings,
/// ChangeProof, or the existing parsed-facts disk cache.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SymbolIndex {
    pub schema_version: u32,
    pub modules: BTreeMap<PathBuf, ModuleSurface>,
    pub references: Vec<ImportReference>,
}

impl SymbolIndex {
    pub fn from_sources<'a>(
        root: &Path,
        sources: impl IntoIterator<Item = SourceModule<'a>>,
    ) -> Result<Self, IndexError> {
        if !root.is_absolute() || normalize_path(root) != root {
            return Err(IndexError::InvalidRoot);
        }

        let mut modules = BTreeMap::new();
        let mut imports = BTreeMap::new();
        for source in sources {
            let path = checked_path(source.path)?;
            if modules.contains_key(&path) {
                return Err(IndexError::DuplicatePath(path));
            }
            let (surface, references) = index_module(&path, source.content);
            imports.insert(path.clone(), references);
            modules.insert(path, surface);
        }

        let mut references = Vec::new();
        for (module, imports) in imports {
            for import in imports {
                let resolution = resolve_observed(root, &modules, &module, &import);
                references.push(ImportReference {
                    module: module.clone(),
                    module_specifier: import.module_specifier,
                    imported_name: import.imported_name,
                    local_name: import.local_name,
                    namespace: import.kind.into(),
                    line_start: import.line_start,
                    line_end: import.line_end,
                    byte_start: import.byte_start,
                    byte_end: import.byte_end,
                    resolution,
                });
            }
        }
        Ok(Self {
            schema_version: 1,
            modules,
            references,
        })
    }
}

fn checked_path(path: &Path) -> Result<PathBuf, IndexError> {
    if path.as_os_str().is_empty()
        || path.to_str().is_none()
        || !path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(IndexError::InvalidPath(path.to_path_buf()));
    }
    Ok(path.components().collect())
}

fn language(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_str()? {
        "ts" => Some("TypeScript"),
        "tsx" => Some("TypeScript React"),
        "js" | "mjs" => Some("JavaScript"),
        "jsx" => Some("JavaScript React"),
        _ => None,
    }
}

fn index_module(path: &Path, content: &str) -> (ModuleSurface, Vec<ImportedSymbolFact>) {
    let unavailable = |coverage| (ModuleSurface::unavailable(coverage), Vec::new());
    let Some(label) = language(path) else {
        return unavailable(ModuleCoverage::UnsupportedLanguage);
    };
    if content.len() > MAX_MODULE_BYTES {
        return unavailable(ModuleCoverage::SourceBudgetExceeded);
    }
    let parsed = ParsedFile::new(content, Some(label));
    let Some(tree) = parsed.tree() else {
        return unavailable(ModuleCoverage::ParseUnavailable);
    };
    if tree.root_node().has_error() {
        return unavailable(ModuleCoverage::SyntaxErrors);
    }
    let Some(facts) = extract_symbol_facts(content, Some(label), tree) else {
        return unavailable(ModuleCoverage::ParseUnavailable);
    };
    let mut surface = ModuleSurface::unavailable(ModuleCoverage::IndexedSurface);
    surface.forwarded_names = facts.re_exports.into_iter().collect();
    surface.wildcard_forwarding = facts.wildcard_re_export;
    for export in facts.exports {
        let symbol = ExportSymbol {
            module: path.to_path_buf(),
            name: export.name,
            namespace: export.kind.into(),
            line_start: export.line_start,
            line_end: export.line_end,
        };
        surface
            .exports
            .entry(symbol.name.clone())
            .or_default()
            .push(symbol);
    }
    (surface, facts.imports)
}

fn resolve_observed(
    root: &Path,
    modules: &BTreeMap<PathBuf, ModuleSurface>,
    importer: &Path,
    import: &ImportedSymbolFact,
) -> Resolution {
    let unresolved = |reason| Resolution::Unresolved { reason };
    // Avoid first-match graph resolution: a unique observed candidate is
    // needed here. Bare package imports and aliases are outside this pilot.
    if !(import.module_specifier.starts_with("./") || import.module_specifier.starts_with("../")) {
        return unresolved(UnresolvedReason::UnsupportedSpecifier);
    }
    let Some(candidates) =
        definitive_local_candidates(&import.module_specifier, &root.join(importer), root)
    else {
        return unresolved(UnresolvedReason::UnsupportedSpecifier);
    };
    let candidates = candidates
        .iter()
        .filter_map(|path| path.strip_prefix(root).ok())
        .filter(|path| modules.contains_key(*path))
        .map(Path::to_path_buf)
        .collect::<BTreeSet<_>>();
    let mut candidates = candidates.into_iter();
    let Some(target) = candidates.next() else {
        return unresolved(UnresolvedReason::ModuleNotObserved);
    };
    if candidates.len() != 0 {
        return Resolution::AmbiguousModules {
            candidates: std::iter::once(target).chain(candidates).collect(),
        };
    }
    let surface = &modules[&target];
    if surface.coverage != ModuleCoverage::IndexedSurface {
        return unresolved(UnresolvedReason::ModuleFactsUnavailable);
    }
    if surface.wildcard_forwarding || surface.forwarded_names.contains(&import.imported_name) {
        return unresolved(UnresolvedReason::ForwardedExport);
    }
    let Some(exports) = surface.exports.get(&import.imported_name) else {
        return unresolved(UnresolvedReason::ExportNotObserved);
    };
    // Mixed namespaces (e.g. merged declarations) are outside the bounded
    // model; a value-only class may also supply a type.
    if exports
        .iter()
        .any(|export| export.namespace != Namespace::from(import.kind))
    {
        return unresolved(UnresolvedReason::NamespaceUncertain);
    }
    match exports.as_slice() {
        [export] => Resolution::UniqueObserved {
            export: export.clone(),
        },
        _ => Resolution::AmbiguousExports {
            candidates: exports.clone(),
        },
    }
}

#[cfg(test)]
mod tests;
