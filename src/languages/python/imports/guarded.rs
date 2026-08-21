use super::{from_import_modules, import_statement_modules};
use crate::analysis::parse::ParsedFile;
use std::collections::HashSet;
use tree_sitter::Node;

pub(in crate::languages::python) fn guarded_optional(parsed: &ParsedFile) -> HashSet<String> {
    let mut imports = HashSet::new();
    if let Some(tree) = parsed.tree()
        && !tree.root_node().has_error()
    {
        collect_guarded_optional(tree.root_node(), parsed.content(), &mut imports);
    }
    imports
}

fn collect_guarded_optional(node: Node<'_>, content: &str, imports: &mut HashSet<String>) {
    if node.kind() == "try_statement"
        && try_absorbs_missing_import(node, content)
        && let Some(body) = node.child_by_field_name("body")
    {
        collect_imports(body, content, imports);
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_guarded_optional(child, content, imports);
    }
}

fn try_absorbs_missing_import(node: Node<'_>, content: &str) -> bool {
    let mut cursor = node.walk();
    for handler in node
        .named_children(&mut cursor)
        .filter(|child| child.kind() == "except_clause")
    {
        let Some(value) = handler.child_by_field_name("value") else {
            return false;
        };
        let caught = exception_names(value, content);
        if caught
            .iter()
            .any(|name| matches!(name.as_str(), "Exception" | "BaseException"))
        {
            return false;
        }
        if caught
            .iter()
            .any(|name| matches!(name.as_str(), "ImportError" | "ModuleNotFoundError"))
        {
            return handler_body(handler).is_some_and(|body| !contains_bare_raise(body, content));
        }
    }
    false
}

fn exception_names(node: Node<'_>, content: &str) -> Vec<String> {
    if node.kind() == "identifier" {
        return node
            .utf8_text(content.as_bytes())
            .ok()
            .map(str::to_string)
            .into_iter()
            .collect();
    }
    if node.kind() != "tuple" {
        return Vec::new();
    }

    let mut names = Vec::new();
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        names.extend(exception_names(child, content));
    }
    names
}

fn handler_body(handler: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = handler.walk();
    handler
        .named_children(&mut cursor)
        .find(|child| child.kind() == "block")
}

fn contains_bare_raise(node: Node<'_>, content: &str) -> bool {
    if node.kind() == "raise_statement"
        && node
            .utf8_text(content.as_bytes())
            .is_ok_and(|text| text.trim() == "raise")
    {
        return true;
    }
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .any(|child| contains_bare_raise(child, content))
}

fn collect_imports(node: Node<'_>, content: &str, imports: &mut HashSet<String>) {
    if node.kind() == "function_definition" {
        return;
    }
    let modules = match node.kind() {
        "import_statement" => node
            .utf8_text(content.as_bytes())
            .ok()
            .map(import_statement_modules),
        "import_from_statement" => node
            .utf8_text(content.as_bytes())
            .ok()
            .map(from_import_modules),
        _ => None,
    };
    imports.extend(modules.into_iter().flatten());

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_imports(child, content, imports);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guarded(content: &str) -> Vec<String> {
        let parsed = ParsedFile::new(content, Some("Python"));
        let mut imports: Vec<_> = guarded_optional(&parsed).into_iter().collect();
        imports.sort();
        imports
    }

    #[test]
    fn import_error_handler_marks_try_body_import_as_optional() {
        let content = "try:\n    from .local import *\nexcept ImportError:\n    pass\n";
        assert_eq!(guarded(content), vec![".local"]);
    }

    #[test]
    fn module_not_found_handler_marks_nested_try_body_import_as_optional() {
        let content = concat!(
            "if enabled:\n",
            "    try:\n",
            "        from .plugin import load\n",
            "    except ModuleNotFoundError:\n",
            "        fallback = None\n",
        );
        assert_eq!(guarded(content), vec![".plugin", ".plugin.load"]);
    }

    #[test]
    fn unrelated_exception_handler_does_not_mark_import_as_optional() {
        let content = "try:\n    from .missing import run\nexcept ValueError:\n    pass\n";
        assert!(guarded(content).is_empty());
    }

    #[test]
    fn bare_reraise_keeps_guarded_import_required() {
        let content = "try:\n    from .missing import run\nexcept ImportError:\n    raise\n";
        assert!(guarded(content).is_empty());
    }

    #[test]
    fn comments_and_strings_do_not_create_guarded_import_facts() {
        let content = concat!(
            "text = 'try: from .fake import value except ImportError: pass'\n",
            "# try: from .comment import value\n",
        );
        assert!(guarded(content).is_empty());
    }

    #[test]
    fn tuple_handler_marks_guarded_import_as_optional() {
        let content = concat!(
            "try:\n",
            "    from .adapter import load\n",
            "except (ImportError, ModuleNotFoundError):\n",
            "    fallback = None\n",
        );
        assert_eq!(guarded(content), vec![".adapter", ".adapter.load"]);
    }

    #[test]
    fn malformed_syntax_does_not_create_guarded_import_facts() {
        let content = "try:\n    from .missing import run\nexcept ImportError\n    pass\n";
        assert!(guarded(content).is_empty());
    }

    #[test]
    fn deferred_function_import_is_not_guarded_by_outer_try() {
        let content = concat!(
            "try:\n",
            "    def load():\n",
            "        from .missing import run\n",
            "except ImportError:\n",
            "    pass\n",
        );
        assert!(guarded(content).is_empty());
    }

    #[test]
    fn qualified_exception_named_import_error_does_not_qualify() {
        let content = concat!(
            "try:\n",
            "    from .missing import run\n",
            "except errors.ImportError:\n",
            "    pass\n",
        );
        assert!(guarded(content).is_empty());
    }
}
