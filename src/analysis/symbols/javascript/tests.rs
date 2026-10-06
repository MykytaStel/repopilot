use super::extract_javascript_symbol_facts;
use crate::analysis::symbols::{JavaScriptSymbolFacts, SymbolKind};
use tree_sitter::Parser;

#[test]
fn symbol_facts_round_trip_without_losing_spans() {
    let source = concat!(
        "export function loadUser() {}\n",
        "export type UserId = string;\n",
        "export { remote as forwarded } from './remote.ts';\n",
        "import { loadUser as load, type UserId } from './api.ts';\n",
    );
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
        .expect("TypeScript grammar");
    let tree = parser.parse(source, None).expect("valid TypeScript tree");

    let facts = extract_javascript_symbol_facts(source, Some("TypeScript"), &tree)
        .expect("supported symbol facts");
    let encoded = serde_json::to_string(&facts).expect("serialize symbol facts");
    let decoded: JavaScriptSymbolFacts =
        serde_json::from_str(&encoded).expect("deserialize symbol facts");

    assert_eq!(decoded, facts);
    assert_eq!(facts.exports[0].name, "UserId");
    assert!(
        facts
            .exports
            .iter()
            .any(|fact| fact.name == "loadUser" && fact.kind == SymbolKind::Value)
    );
    assert_eq!(facts.re_exports, vec!["forwarded"]);
    assert!(facts.imports.iter().any(|fact| {
        fact.imported_name == "loadUser"
            && fact.local_name == "load"
            && fact.module_specifier == "./api.ts"
            && fact.line_start == 4
            && fact.byte_end > fact.byte_start
    }));
}

#[test]
fn default_facts_round_trip_preserves_distinct_import_occurrences() {
    let source = "export default () => 42;\nimport first, { default as second } from './api';\n";
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
        .unwrap();
    let tree = parser.parse(source, None).unwrap();
    let facts = extract_javascript_symbol_facts(source, Some("TypeScript"), &tree).unwrap();
    let encoded = serde_json::to_string(&facts).unwrap();
    let decoded: JavaScriptSymbolFacts = serde_json::from_str(&encoded).unwrap();
    assert_eq!(facts, decoded);
    assert_eq!(facts.exports[0].name, "default");
    assert_eq!(facts.imports.len(), 2);
    assert!(
        facts
            .imports
            .iter()
            .all(|import| import.imported_name == "default")
    );
    assert_ne!(facts.imports[0].local_name, facts.imports[1].local_name);
    assert_ne!(facts.imports[0].byte_start, facts.imports[1].byte_start);
}
