use super::model::ArchitectureAnalysis;
use crate::audits::traits::ProjectAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::scan::config::ScanConfig;
use crate::scan::facts::{FileContentProvider, ScanFacts};
use std::cell::RefCell;
use std::path::PathBuf;
use tree_sitter::{Node, Parser};

thread_local! {
    static RUST_PARSER: RefCell<Parser> = RefCell::new({
        let mut p = Parser::new();
        p.set_language(&tree_sitter_rust::LANGUAGE.into()).expect("rust grammar");
        p
    });
    static TSX_PARSER: RefCell<Parser> = RefCell::new({
        let mut p = Parser::new();
        p.set_language(&tree_sitter_typescript::LANGUAGE_TSX.into()).expect("tsx grammar");
        p
    });
    static PYTHON_PARSER: RefCell<Parser> = RefCell::new({
        let mut p = Parser::new();
        p.set_language(&tree_sitter_python::LANGUAGE.into()).expect("python grammar");
        p
    });
}

pub struct DeepNestingAudit;

impl ProjectAudit for DeepNestingAudit {
    fn audit(&self, facts: &ScanFacts, config: &ScanConfig) -> Vec<Finding> {
        ArchitectureAnalysis::from_facts(facts)
            .production_files()
            .filter_map(|file| {
                let content = FileContentProvider.content(file.facts)?;
                let ext = file
                    .path()
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                let depth = compute_ast_nesting(&content, ext);
                if depth > config.max_directory_depth {
                    Some(build_finding(
                        file.path().to_path_buf(),
                        depth,
                        config.max_directory_depth,
                    ))
                } else {
                    None
                }
            })
            .collect()
    }
}

fn compute_ast_nesting(content: &str, ext: &str) -> usize {
    let tree = match ext {
        "rs" => RUST_PARSER.with(|cell| {
            let mut p = cell.borrow_mut();
            p.reset();
            p.parse(content, None)
        }),
        "ts" | "tsx" | "js" | "jsx" | "mts" | "mjs" => TSX_PARSER.with(|cell| {
            let mut p = cell.borrow_mut();
            p.reset();
            p.parse(content, None)
        }),
        "py" => PYTHON_PARSER.with(|cell| {
            let mut p = cell.borrow_mut();
            p.reset();
            p.parse(content, None)
        }),
        _ => return 0,
    };

    let Some(tree) = tree else {
        return 0;
    };

    let mut max_depth = 0;
    walk_nesting(tree.root_node(), 0, &mut max_depth, ext);
    max_depth
}

fn is_nesting_node(node_kind: &str, ext: &str) -> bool {
    match ext {
        "rs" => matches!(
            node_kind,
            "if_expression"
                | "for_expression"
                | "while_expression"
                | "loop_expression"
                | "match_expression"
        ),
        "ts" | "tsx" | "js" | "jsx" | "mts" | "mjs" => matches!(
            node_kind,
            "if_statement"
                | "for_statement"
                | "for_in_statement"
                | "while_statement"
                | "do_statement"
                | "switch_statement"
                | "catch_clause"
        ),
        "py" => matches!(
            node_kind,
            "if_statement"
                | "for_statement"
                | "while_statement"
                | "with_statement"
                | "try_statement"
        ),
        _ => false,
    }
}

fn walk_nesting(node: Node<'_>, current_depth: usize, max_depth: &mut usize, ext: &str) {
    let is_nest = is_nesting_node(node.kind(), ext);
    let next_depth = if is_nest {
        let d = current_depth + 1;
        *max_depth = (*max_depth).max(d);
        d
    } else {
        current_depth
    };

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_nesting(child, next_depth, max_depth, ext);
    }
}

fn build_finding(path: PathBuf, depth: usize, threshold: usize) -> Finding {
    Finding {
        id: String::new(),
        rule_id: "architecture.deep-nesting".to_string(),
        recommendation: Finding::recommendation_for_rule_id("architecture.deep-nesting"),
        title: "Deep control flow nesting detected".to_string(),
        description: format!(
            "This file contains control flow blocks nested {depth} levels deep, exceeding the threshold \
             of {threshold}. Deep control flow nesting makes code hard to read and maintain."
        ),
        category: FindingCategory::Architecture,
        severity: Severity::Low,
        confidence: Default::default(),
        evidence: vec![Evidence {
            path,
            line_start: 1,
            line_end: None,
            snippet: format!("Control flow nesting depth: {depth}; threshold is {threshold}."),
        }],
        workspace_package: None,
        docs_url: None,
        provenance: Default::default(),
        risk: Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::facts::FileFacts;

    #[test]
    fn ignores_rule_fixtures_when_calculating_deepest_path() {
        let facts = ScanFacts {
            root_path: PathBuf::from("."),
            files: vec![
                file_facts_with_content("./src/domain/user.ts", "export const x = 1;"),
                file_facts_with_content(
                    "./tests/fixtures/rules/security.secret-candidate/true_positive_env_value/src/config.ts",
                    r#"
                        if (a) {
                            if (b) {
                                if (c) {
                                    if (d) {
                                        if (e) {
                                            if (f) {
                                                console.log("nested");
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    "#,
                ),
            ],
            ..ScanFacts::default()
        };

        let findings = audit_with_depth(&facts, 5);

        assert!(
            findings.is_empty(),
            "rule fixture paths should not become production architecture findings"
        );
    }

    #[test]
    fn ignores_test_paths_even_when_they_are_deeper_than_source_paths() {
        let nested_code = r#"
            if (a) {
                if (b) {
                    if (c) {
                        if (d) {
                            if (e) {
                                if (f) {
                                    console.log("nested");
                                }
                            }
                        }
                    }
                }
            }
        "#;
        let facts = ScanFacts {
            root_path: PathBuf::from("."),
            files: vec![
                file_facts_with_content("./src/service.ts", "export const x = 1;"),
                file_facts_with_content("./tests/unit/service.test.ts", nested_code),
            ],
            ..ScanFacts::default()
        };

        let findings = audit_with_depth(&facts, 5);

        assert!(
            findings.is_empty(),
            "test paths are allowed to be deeper than production source paths"
        );
    }

    #[test]
    fn ignores_docs_examples_generated_vendor_and_build_paths() {
        let nested_code = r#"
            if (a) {
                if (b) {
                    if (c) {
                        if (d) {
                            if (e) {
                                if (f) {
                                    console.log("nested");
                                }
                            }
                        }
                    }
                }
            }
        "#;
        let facts = ScanFacts {
            root_path: PathBuf::from("."),
            files: vec![
                file_facts_with_content(
                    "./docs/reference/api/v1/generated/client/config.ts",
                    nested_code,
                ),
                file_facts_with_content(
                    "./examples/react-native/deep/sample/src/App.tsx",
                    nested_code,
                ),
                file_facts_with_content(
                    "./src/generated/openapi/client/v1/types.generated.ts",
                    nested_code,
                ),
                file_facts_with_content(
                    "./vendor/company/package/deep/source/file.ts",
                    nested_code,
                ),
                file_facts_with_content(
                    "./target/debug/build/package/out/generated.rs",
                    nested_code,
                ),
                file_facts_with_content("./dist/assets/js/chunks/deep/file.js", nested_code),
            ],
            ..ScanFacts::default()
        };

        let findings = audit_with_depth(&facts, 5);

        assert!(findings.is_empty());
    }

    #[test]
    fn reports_deep_production_path() {
        let production_path = "./src/handler.ts";
        let nested_code = r#"
            if (a) {
                if (b) {
                    if (c) {
                        if (d) {
                            if (e) {
                                if (f) {
                                    console.log("nested");
                                }
                            }
                        }
                    }
                }
            }
        "#;
        let facts = ScanFacts {
            root_path: PathBuf::from("."),
            files: vec![file_facts_with_content(production_path, nested_code)],
            ..ScanFacts::default()
        };

        let findings = audit_with_depth(&facts, 5);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "architecture.deep-nesting");
        assert_eq!(findings[0].evidence[0].path, PathBuf::from(production_path));
    }

    fn audit_with_depth(facts: &ScanFacts, max_directory_depth: usize) -> Vec<Finding> {
        let audit = DeepNestingAudit;
        let config = ScanConfig {
            max_directory_depth,
            ..ScanConfig::default()
        };

        audit.audit(facts, &config)
    }

    fn file_facts_with_content(path: &str, content: &str) -> FileFacts {
        FileFacts {
            path: PathBuf::from(path),
            language: Some("TypeScript".to_string()),
            non_empty_lines: content.lines().count(),
            branch_count: 0,
            imports: Vec::new(),
            content: Some(content.to_string()),
            has_inline_tests: false,
        }
    }
}
