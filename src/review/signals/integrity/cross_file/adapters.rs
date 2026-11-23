use crate::analysis::symbols::SymbolKind;
use crate::review::signals::api_contract::extract_javascript_symbol_facts;
use crate::review::signals::content::ReviewSource;
use std::collections::BTreeMap;

#[path = "go_scope.rs"]
mod go_scope;

#[path = "python_scope.rs"]
mod python_scope;
pub(super) use python_scope::shadowed_in_test;

#[derive(Debug)]
pub(super) struct DeclaredHelpers {
    pub lines: BTreeMap<String, usize>,
    pub package_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ImportedHelper {
    pub module_specifier: String,
    pub imported_name: String,
    pub local_name: String,
}

pub(super) fn declared_helpers(source: &ReviewSource) -> Option<DeclaredHelpers> {
    let label = source.language_label()?;
    match label {
        "TypeScript" | "TypeScript React" | "JavaScript" | "JavaScript React" => {
            let facts = extract_javascript_symbol_facts(source)?;
            Some(DeclaredHelpers {
                lines: facts
                    .exports
                    .into_iter()
                    .filter(|export| export.kind == SymbolKind::Value)
                    .map(|export| (export.name, export.line_start))
                    .collect(),
                package_name: None,
            })
        }
        "Python" => Some(DeclaredHelpers {
            lines: top_level_functions(source, "function_definition"),
            package_name: None,
        }),
        "Go" => Some(DeclaredHelpers {
            lines: top_level_functions(source, "function_declaration"),
            package_name: go_package_name(source),
        }),
        _ => None,
    }
}

pub(super) fn imported_helpers(source: &ReviewSource) -> Option<Vec<ImportedHelper>> {
    match source.language_label()? {
        "TypeScript" | "TypeScript React" | "JavaScript" | "JavaScript React" => {
            let facts = extract_javascript_symbol_facts(source)?;
            Some(
                facts
                    .imports
                    .into_iter()
                    .filter(|import| import.kind == SymbolKind::Value)
                    .map(|import| ImportedHelper {
                        module_specifier: import.module_specifier,
                        imported_name: import.imported_name,
                        local_name: import.local_name,
                    })
                    .collect(),
            )
        }
        "Python" => Some(python_scope::imports(source)),
        "Go" => Some(Vec::new()),
        _ => None,
    }
}

pub(super) fn go_same_package_helpers(
    source: &ReviewSource,
    caller_path: &std::path::Path,
    helper_path: &std::path::Path,
    helper_package: Option<&str>,
    helper_name: &str,
) -> bool {
    source.language_label() == Some("Go")
        && caller_path.parent() == helper_path.parent()
        && go_package_name(source).as_deref() == helper_package
        && helper_package.is_some()
        && !go_scope::name_is_locally_bound(source, helper_name)
}

fn top_level_functions(source: &ReviewSource, kind: &str) -> BTreeMap<String, usize> {
    let Some(tree) = source.tree() else {
        return BTreeMap::new();
    };
    let root = tree.root_node();
    let mut names = BTreeMap::new();
    let mut cursor = root.walk();
    for child in root.named_children(&mut cursor) {
        let definition = if child.kind() == "decorated_definition" {
            child.child_by_field_name("definition").unwrap_or(child)
        } else {
            child
        };
        if definition.kind() == kind
            && let Some(name) = definition.child_by_field_name("name")
            && let Ok(name) = name.utf8_text(source.content().as_bytes())
        {
            names.insert(name.to_string(), definition.start_position().row + 1);
        }
    }
    names
}

fn go_package_name(source: &ReviewSource) -> Option<String> {
    let root = source.tree()?.root_node();
    let mut cursor = root.walk();
    let package = root
        .named_children(&mut cursor)
        .find(|node| node.kind() == "package_clause")?;
    package
        .utf8_text(source.content().as_bytes())
        .ok()?
        .split_whitespace()
        .nth(1)
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_helpers_keep_imported_aliases() {
        let source = ReviewSource::new(
            "from .checks import verify as assert_valid, other\n".into(),
            Some("Python".into()),
        );
        assert_eq!(
            python_scope::imports(&source),
            vec![
                ImportedHelper {
                    module_specifier: ".checks".into(),
                    imported_name: "other".into(),
                    local_name: "other".into(),
                },
                ImportedHelper {
                    module_specifier: ".checks".into(),
                    imported_name: "verify".into(),
                    local_name: "assert_valid".into(),
                },
            ]
        );
    }

    #[test]
    fn language_helpers_are_only_top_level_definitions() {
        let python = ReviewSource::new(
            "def helper(): pass\nclass C:\n    def nested(): pass\n".into(),
            Some("Python".into()),
        );
        assert_eq!(
            declared_helpers(&python).unwrap().lines,
            BTreeMap::from([("helper".to_string(), 1)])
        );
        let go = ReviewSource::new(
            "package app\nfunc Verify() {}\nfunc (s *Suite) Method() {}\n".into(),
            Some("Go".into()),
        );
        assert_eq!(
            declared_helpers(&go).unwrap().lines,
            BTreeMap::from([("Verify".to_string(), 2)])
        );
    }
}
