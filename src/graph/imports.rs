use crate::analysis::parse::ParsedFile;
use std::collections::HashSet;
use tree_sitter::Tree;

mod common;
mod go;
mod jvm;
pub mod lines;
mod python;
mod rust;
mod ts;

/// Extracts raw import strings from file content based on language.
/// Returns a deduplicated list.
///
/// Standalone entry point (tests, callers without a parse view). It builds a
/// throwaway [`ParsedFile`] and delegates to [`extract_imports_from`], so the
/// AST languages route through the same shared tree-sitter parsers as the scan.
pub fn extract_imports(content: &str, language: Option<&str>) -> Vec<String> {
    extract_imports_from(&ParsedFile::new(content, language), language)
}

/// Extracts raw import strings from an already-built parse view.
///
/// The scan pipeline calls this with the same [`ParsedFile`] the per-file audits
/// receive, so a file's syntax tree is produced once and shared between import
/// extraction and the AST audits rather than parsed separately for each.
pub(crate) fn extract_imports_from(parsed: &ParsedFile, language: Option<&str>) -> Vec<String> {
    let content = parsed.content();
    let set: HashSet<String> = match language {
        Some("Rust") => from_tree(parsed, |tree| rust::extract(tree, content)),
        Some("TypeScript")
        | Some("TypeScript React")
        | Some("JavaScript")
        | Some("JavaScript React") => from_tree(parsed, |tree| ts::extract(tree, content)),
        Some("Python") => from_tree(parsed, |tree| python::extract(tree, content)),
        Some("Go") => go::extract(content),
        Some("Java") => jvm::extract_java(content),
        Some("Kotlin") => jvm::extract_kotlin(content),
        _ => return Vec::new(),
    };
    set.into_iter().collect()
}

/// Runs `visit` over the shared syntax tree, yielding no imports when the file
/// has no parseable grammar or tree-sitter could not produce a tree.
fn from_tree(parsed: &ParsedFile, visit: impl FnOnce(&Tree) -> HashSet<String>) -> HashSet<String> {
    parsed.tree().map(visit).unwrap_or_default()
}
