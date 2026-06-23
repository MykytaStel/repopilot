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

/// Imports that are *deferred* — present only inside a function body, never at
/// module scope. Currently Python-only (the language whose idiom is to defer an
/// import to break a load-time cycle). These are real edges in the coupling
/// graph but are subtracted by cycle detection. Other languages have none.
pub(crate) fn extract_deferred_imports_from(
    parsed: &ParsedFile,
    language: Option<&str>,
) -> Vec<String> {
    let content = parsed.content();
    let set: HashSet<String> = match language {
        Some("Python") => from_tree(parsed, |tree| python::extract_deferred(tree, content)),
        _ => return Vec::new(),
    };
    set.into_iter().collect()
}

/// Runs `visit` over the shared syntax tree, yielding no imports when the file
/// has no parseable grammar or tree-sitter could not produce a tree.
fn from_tree(parsed: &ParsedFile, visit: impl FnOnce(&Tree) -> HashSet<String>) -> HashSet<String> {
    parsed.tree().map(visit).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deferred(content: &str) -> Vec<String> {
        let mut d = extract_deferred_imports_from(
            &ParsedFile::new(content, Some("Python")),
            Some("Python"),
        );
        d.sort();
        d
    }

    #[test]
    fn module_level_python_import_is_not_deferred() {
        let content = "from app.models import User\n\ndef f():\n    return User\n";
        assert!(extract_imports(content, Some("Python")).contains(&"app.models".to_string()));
        assert!(deferred(content).is_empty());
    }

    #[test]
    fn function_body_python_import_is_deferred() {
        let content = "def handler():\n    from app.permissions import policy\n    return policy\n";
        // Still a real edge (present in the full import set)…
        assert!(extract_imports(content, Some("Python")).contains(&"app.permissions".to_string()));
        // …but reported as deferred so cycle detection can subtract it (both the
        // module and the imported-submodule candidate the resolver may match on).
        assert_eq!(
            deferred(content),
            vec![
                "app.permissions".to_string(),
                "app.permissions.policy".to_string()
            ]
        );
    }

    #[test]
    fn import_used_both_eagerly_and_lazily_is_not_deferred() {
        let content = "from app.models import User\n\ndef f():\n    from app.models import User\n    return User\n";
        // The eager import keeps the edge, so it must not be reported as deferred.
        assert!(deferred(content).is_empty());
    }

    #[test]
    fn non_python_languages_have_no_deferred_imports() {
        let content = "use crate::app::models::User;\nfn f() { use crate::app::other::X; }\n";
        assert!(
            extract_deferred_imports_from(&ParsedFile::new(content, Some("Rust")), Some("Rust"))
                .is_empty()
        );
    }
}
