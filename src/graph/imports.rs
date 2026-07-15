use crate::analysis::parse::ParsedFile;
use crate::languages::imports_for_label;

pub mod lines;

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
/// The per-language extractors live on the language frontend registry
/// (`languages::*::imports`); this dispatches to the frontend claiming the
/// label. The scan pipeline calls it with the same [`ParsedFile`] the
/// per-file audits receive, so a file's syntax tree is produced once and
/// shared between import extraction and the AST audits.
pub(crate) fn extract_imports_from(parsed: &ParsedFile, language: Option<&str>) -> Vec<String> {
    match language.and_then(imports_for_label) {
        Some(extractor) => (extractor.eager)(parsed).into_iter().collect(),
        None => Vec::new(),
    }
}

/// Imports that are real edges in the coupling graph but must be subtracted by
/// cycle detection because they create no module-load dependency:
/// - **Python**: imports deferred into a function body (the idiom for breaking a
///   load-time cycle), present only inside a function, never at module scope.
/// - **TypeScript/JavaScript**: `import type` / `export type` imports, which the
///   compiler erases, so they are type-only and never run at load time.
///
/// These stay full edges for coupling, fan-out, and dead-module analysis; only
/// cycle detection subtracts them. Languages whose frontend registers no
/// deferred extractor have none.
pub(crate) fn extract_deferred_imports_from(
    parsed: &ParsedFile,
    language: Option<&str>,
) -> Vec<String> {
    match language
        .and_then(imports_for_label)
        .and_then(|e| e.deferred)
    {
        Some(deferred) => deferred(parsed).into_iter().collect(),
        None => Vec::new(),
    }
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

    fn type_only(content: &str) -> Vec<String> {
        let mut d = extract_deferred_imports_from(
            &ParsedFile::new(content, Some("TypeScript")),
            Some("TypeScript"),
        );
        d.sort();
        d
    }

    #[test]
    fn ts_import_type_is_type_only_but_still_a_full_edge() {
        let content = "import type { User } from \"./types\";\nimport { run } from \"./run\";\n";
        // Both remain real edges for coupling / fan-out…
        let imports = extract_imports(content, Some("TypeScript"));
        assert!(imports.contains(&"./types".to_string()));
        assert!(imports.contains(&"./run".to_string()));
        // …but only the erased `import type` is subtracted from cycle detection.
        assert_eq!(type_only(content), vec!["./types".to_string()]);
    }

    #[test]
    fn ts_value_and_mixed_imports_are_not_type_only() {
        // A plain value import and a mixed `{ type A, B }` both execute the module.
        let content = "import { A } from \"./a\";\nimport { type T, b } from \"./b\";\n";
        assert!(type_only(content).is_empty(), "{:?}", type_only(content));
    }

    #[test]
    fn ts_all_inline_type_specifiers_are_type_only() {
        let content = "import { type A, type B } from \"./types\";\n\
             export { type C } from \"./config\";\n";

        assert_eq!(
            type_only(content),
            vec!["./config".to_string(), "./types".to_string()]
        );
    }

    #[test]
    fn ts_inline_type_and_value_specifiers_keep_runtime_edge() {
        let content = "import { type A, b } from \"./module\";\n";
        assert!(type_only(content).is_empty());
    }

    #[test]
    fn ts_export_type_reexport_is_type_only_but_value_barrel_is_not() {
        let content = "export type { Cfg } from \"./cfg\";\nexport { thing } from \"./thing\";\n";
        assert_eq!(type_only(content), vec!["./cfg".to_string()]);
    }

    #[test]
    fn ts_module_imported_both_type_and_value_is_not_type_only() {
        // The value import keeps the runtime edge, so the module is not type-only.
        let content = "import type { T } from \"./x\";\nimport { run } from \"./x\";\n";
        assert!(type_only(content).is_empty());
    }
}
